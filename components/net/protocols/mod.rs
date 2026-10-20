/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::collections::hash_map::Entry;
use std::future;
use std::future::Future;
use std::ops::Bound;
use std::pin::Pin;
use std::sync::Arc;

use headers::Range;
use http::StatusCode;
use log::error;
use net_traits::blob_url_store::UrlWithBlobClaim;
use net_traits::filemanager_thread::RelativePos;
use net_traits::http_status::HttpStatus;
use net_traits::request::Request;
use net_traits::response::{Response, ResponseBody};
use net_traits::{DiscardFetch, NetworkError, ResourceFetchTiming};
use rustc_hash::FxHashMap;
use servo_url::ServoUrl;
use url::Position;

use crate::fetch::methods::{DoneChannel, FetchContext, RangeRequestBounds, fetch};

mod blob;
mod data;
mod file;

use blob::BlobProtocolHander;
use data::DataProtocolHander;
use file::FileProtocolHander;

type FutureResponse<'a> = Pin<Box<dyn Future<Output = Response> + Send + 'a>>;

// The set of schemes that can't be registered.
static FORBIDDEN_SCHEMES: [&str; 4] = ["http", "https", "chrome", "about"];

pub trait ProtocolHandler: Send + Sync {
    /// A list of schema-less URLs that can be resolved against this handler's
    /// scheme. These URLs will be granted access to the `navigator.chora`
    /// interface to perform privileged operations that manipulate Chora internals.
    fn privileged_paths(&self) -> &'static [&'static str] {
        &[]
    }

    /// Triggers the load of a resource for this protocol and returns a future
    /// that will produce a Response. Even if the protocol is not backed by a
    /// http endpoint, it is recommended to a least provide:
    /// - A relevant status code.
    /// - A Content Type.
    fn load<'a>(
        &'a self,
        request: &'a mut Request,
        done_chan: &mut DoneChannel,
        context: &FetchContext,
    ) -> FutureResponse<'a>;

    /// Specify if resources served by that protocol can be retrieved
    /// with `fetch()` without no-cors mode to allow the caller direct
    /// access to the resource content.
    fn is_fetchable(&self) -> bool {
        false
    }

    /// Specify if this custom protocol can be used in a [secure context]
    ///
    /// Note: this only works for bypassing mixed content checks right now
    ///
    /// [secure context]: https://developer.mozilla.org/en-US/docs/Web/Security/Secure_Contexts
    fn is_secure(&self) -> bool {
        false
    }
}

#[derive(Default)]
pub struct ProtocolRegistry {
    pub(crate) handlers: FxHashMap<String, Box<dyn ProtocolHandler>>, // Maps scheme -> handler
}

#[derive(Clone, Copy, Debug)]
pub enum ProtocolRegisterError {
    ForbiddenScheme,
    SchemeAlreadyRegistered,
}

impl ProtocolRegistry {
    pub fn with_internal_protocols() -> Self {
        let mut registry = Self::default();
        // We just created a new registry, and know that we aren't using
        // any forbidden schemes, so this should never panic.
        registry
            .register("data", DataProtocolHander::default())
            .expect("Infallible");
        registry
            .register("blob", BlobProtocolHander::default())
            .expect("Infallible");
        registry
            .register("file", FileProtocolHander::default())
            .expect("Infallible");
        registry
    }

    /// Do not allow users to enter an arbitrary protocol as this can lead to slowdowns.
    pub fn register(
        &mut self,
        scheme: &str,
        handler: impl ProtocolHandler + 'static,
    ) -> Result<(), ProtocolRegisterError> {
        if FORBIDDEN_SCHEMES.contains(&scheme) {
            error!("Protocol handler for '{scheme}' is not allowed to be registered.");
            return Err(ProtocolRegisterError::ForbiddenScheme);
        }

        if let Entry::Vacant(entry) = self.handlers.entry(scheme.into()) {
            entry.insert(Box::new(handler));
            Ok(())
        } else {
            error!("Protocol handler for '{scheme}' is already registered.");
            Err(ProtocolRegisterError::SchemeAlreadyRegistered)
        }
    }

    pub fn register_page_content_handler(
        &mut self,
        scheme: String,
        url: String,
    ) -> Result<(), ProtocolRegisterError> {
        self.register(
            &scheme.clone(),
            WebPageContentProtocolHandler { url, scheme },
        )
    }

    pub fn get(&self, scheme: &str) -> Option<&dyn ProtocolHandler> {
        self.handlers.get(scheme).map(|e| e.as_ref())
    }

    pub fn merge(&mut self, mut other: ProtocolRegistry) {
        for (scheme, handler) in other.handlers.drain() {
            if FORBIDDEN_SCHEMES.contains(&scheme.as_str()) {
                error!("Protocol handler for '{scheme}' is not allowed to be registered.");
                continue;
            }

            self.handlers.entry(scheme).or_insert(handler);
        }
    }

    pub fn is_fetchable(&self, scheme: &str) -> bool {
        self.handlers
            .get(scheme)
            .is_some_and(|handler| handler.is_fetchable())
    }

    pub fn is_secure(&self, scheme: &str) -> bool {
        self.handlers
            .get(scheme)
            .is_some_and(|handler| handler.is_secure())
    }

    /// Return true if the registered handler for this scheme is marked as secure.
    pub fn is_secure_scheme(&self, scheme: &str) -> bool {
        self.is_secure(scheme)
    }

    /// Register a custom protocol scheme handled by a synchronous closure.
    pub fn register_sync_closure(
        &mut self,
        scheme: &str,
        fetchable: bool,
        secure: bool,
        handler: Arc<dyn Fn(&str) -> (u16, String, Vec<u8>) + Send + Sync>,
    ) -> Result<(), ProtocolRegisterError> {
        let handler = SyncClosureProtocolHandler::new(handler)
            .with_fetchable(fetchable)
            .with_secure(secure);
        self.register(scheme, handler)
    }

    pub fn privileged_urls(&self) -> Vec<ServoUrl> {
        self.handlers
            .iter()
            .flat_map(|(scheme, handler)| {
                let paths = handler.privileged_paths();
                paths
                    .iter()
                    .filter_map(move |path| ServoUrl::parse(&format!("{scheme}:{path}")).ok())
            })
            .collect()
    }
}

pub type SyncProtocolClosure = Arc<dyn Fn(&str) -> (u16, String, Vec<u8>) + Send + Sync>;

/// A protocol handler backed by a synchronous closure returning `(status_code, content_type, body_bytes)`.
///
/// Enables host embedders to register custom protocols (such as `qualia://` or `webizen://`)
/// with configurable fetchability and secure context flags.
#[derive(Clone)]
pub struct SyncClosureProtocolHandler {
    handler: SyncProtocolClosure,
    fetchable: bool,
    secure: bool,
    privileged_paths: &'static [&'static str],
}

impl SyncClosureProtocolHandler {
    pub fn new(handler: SyncProtocolClosure) -> Self {
        Self {
            handler,
            fetchable: true,
            secure: true,
            privileged_paths: &[],
        }
    }

    pub fn with_fetchable(mut self, fetchable: bool) -> Self {
        self.fetchable = fetchable;
        self
    }

    pub fn with_secure(mut self, secure: bool) -> Self {
        self.secure = secure;
        self
    }

    pub fn with_privileged_paths(mut self, paths: &'static [&'static str]) -> Self {
        self.privileged_paths = paths;
        self
    }
}

impl ProtocolHandler for SyncClosureProtocolHandler {
    fn privileged_paths(&self) -> &'static [&'static str] {
        self.privileged_paths
    }

    fn load<'a>(
        &'a self,
        request: &'a mut Request,
        _done_chan: &mut DoneChannel,
        _context: &FetchContext,
    ) -> FutureResponse<'a> {
        let url = request.current_url();
        let (status_code, content_type, body_bytes) = (self.handler)(url.as_str());

        let mut response =
            Response::new(url, ResourceFetchTiming::new(request.timing_type()));
        if let Ok(code) = StatusCode::from_u16(status_code) {
            response.status = code.into();
        } else {
            response.status = HttpStatus::new_error();
        }

        if let Ok(content_type_val) = http::HeaderValue::from_str(&content_type) {
            response
                .headers
                .insert(http::header::CONTENT_TYPE, content_type_val);
        }

        *response.body.lock() = ResponseBody::Done(body_bytes);
        Box::pin(future::ready(response))
    }

    fn is_fetchable(&self) -> bool {
        self.fetchable
    }

    fn is_secure(&self) -> bool {
        self.secure
    }
}

struct WebPageContentProtocolHandler {
    url: String,
    scheme: String,
}

impl ProtocolHandler for WebPageContentProtocolHandler {
    /// <https://html.spec.whatwg.org/multipage/#protocol-handler-invocation>
    fn load<'a>(
        &'a self,
        request: &'a mut Request,
        _done_chan: &mut DoneChannel,
        context: &FetchContext,
    ) -> FutureResponse<'a> {
        let mut url = request.current_url();
        // Step 1. Assert: inputURL's scheme is normalizedScheme.
        assert!(url.scheme() == self.scheme);
        // Step 2. Set the username given inputURL and the empty string.
        //
        // Ignore errors if no username can be set, which depending on the
        // URL in the scheme handler might be bogus. For example, with a
        // `mailto:` handler, it doesn't have a base. See the
        // documentation at [`url::Url::set_username`]
        let _ = url.set_username("");

        // Step 3. Set the password given inputURL and the empty string.
        //
        // Ignore errors if no password can be set, which depending on the
        // URL in the scheme handler might be bogus. For example, with a
        // `mailto:` handler, it doesn't have a base. See the
        // documentation at [`url::Url::set_password`]
        let _ = url.set_password(None);

        // Step 4. Let inputURLString be the serialization of inputURL.
        // Step 5. Let encodedURL be the result of running UTF-8 percent-encode on inputURLString using the component percent-encode set.
        //
        // Url is already UTF-8, so encoding isn't required
        let encoded_url = &url[Position::AfterScheme..][1..];
        // Step 6. Let handlerURLString be normalizedURLString.
        // Step 7. Replace the first instance of "%s" in handlerURLString with encodedURL.
        let handler_url_string = self.url.replacen("%s", encoded_url, 1);
        // Step 8. Let resultURL be the result of parsing handlerURLString.
        let Ok(result_url) = ServoUrl::parse(&handler_url_string) else {
            return Box::pin(future::ready(Response::network_error(
                NetworkError::ProtocolHandlerSubstitutionError,
            )));
        };
        // Ensure we did a proper substitution with a HTTP result
        assert!(matches!(result_url.scheme(), "http" | "https"));
        // Step 9. Navigate an appropriate navigable to resultURL.
        request
            .url_list
            .push(UrlWithBlobClaim::new(result_url, None));
        let request2 = request.clone();
        let context2 = context.clone();
        Box::pin(async move { fetch(request2, &mut DiscardFetch, &context2).await })
    }
}

/// Test if the URL is potentially trustworthy or the custom protocol is registered as secure
pub fn is_url_potentially_trustworthy(
    protocol_registry: &ProtocolRegistry,
    url: &ServoUrl,
) -> bool {
    url.is_potentially_trustworthy() || protocol_registry.is_secure(url.scheme())
}

pub fn range_not_satisfiable_error(response: &mut Response) {
    response.status = StatusCode::RANGE_NOT_SATISFIABLE.into();
}

/// Get the range bounds if the `Range` header is present.
pub fn get_range_request_bounds(range: Option<Range>, len: u64) -> RangeRequestBounds {
    if let Some(ref range) = range {
        let (start, end) = match range.satisfiable_ranges(len).next() {
            Some((Bound::Included(start), Bound::Unbounded)) => (start, None),
            Some((Bound::Included(start), Bound::Included(end))) => {
                // `end` should be less or equal to `start`.
                (start, Some(i64::max(start as i64, end as i64)))
            },
            Some((Bound::Unbounded, Bound::Included(offset))) => {
                return RangeRequestBounds::Pending(offset);
            },
            _ => (0, None),
        };
        RangeRequestBounds::Final(RelativePos::from_opts(Some(start as i64), end))
    } else {
        RangeRequestBounds::Final(RelativePos::from_opts(Some(0), None))
    }
}

pub fn partial_content(response: &mut Response) {
    response.status = StatusCode::PARTIAL_CONTENT.into();
}

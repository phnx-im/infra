// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use tonic::{Request, Status, service::Interceptor};

// Extracts the IP s.t. the key extractor of the rate limiter (governor) can use it
#[derive(Debug, Clone)]
pub struct ConnectInfoInterceptor;

impl Interceptor for ConnectInfoInterceptor {
    fn call(&mut self, mut request: Request<()>) -> Result<Request<()>, Status> {
        let metadata = request.metadata();
        if metadata.contains_key("x-real-ip")
            || metadata.contains_key("x-forwarded-for")
            || metadata.contains_key("forwarded")
        {
            return Ok(request);
        }
        // fallback to remote_addr: this won't work behind a reverse proxy
        let addr = request
            .remote_addr()
            .ok_or_else(|| Status::internal("failed to extract remote address from request"))?;
        request.extensions_mut().insert(addr);
        Ok(request)
    }
}

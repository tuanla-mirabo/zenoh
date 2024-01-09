//
// Copyright (c) 2023 ZettaScale Technology
//
// This program and the accompanying materials are made available under the
// terms of the Eclipse Public License 2.0 which is available at
// http://www.eclipse.org/legal/epl-2.0, or the Apache License, Version 2.0
// which is available at https://www.apache.org/licenses/LICENSE-2.0.
//
// SPDX-License-Identifier: EPL-2.0 OR Apache-2.0
//
// Contributors:
//   ZettaScale Zenoh Team, <zenoh@zettascale.tech>
//
use super::Primitives;
use crate::net::routing::{
    dispatcher::face::Face,
    interceptor::{InterceptTrait, InterceptsChain},
    RoutingContext,
};
use std::any::Any;
use zenoh_link::Link;
use zenoh_protocol::network::{NetworkBody, NetworkMessage};
use zenoh_result::ZResult;
use zenoh_transport::unicast::TransportUnicast;
use zenoh_transport::TransportPeerEventHandler;

pub struct DeMux {
    face: Face,
    pub(crate) transport: Option<TransportUnicast>,
    pub(crate) intercept: InterceptsChain,
}

impl DeMux {
    pub(crate) fn new(
        face: Face,
        transport: Option<TransportUnicast>,
        intercept: InterceptsChain,
    ) -> Self {
        Self {
            face,
            transport,
            intercept,
        }
    }
}

impl TransportPeerEventHandler for DeMux {
    #[inline]
    fn handle_message(&self, mut msg: NetworkMessage) -> ZResult<()> {
        if !self.intercept.intercepts.is_empty() {
            let ctx = RoutingContext::new_in(msg, self.face.clone());
            let ctx = match self.intercept.intercept(ctx) {
                Some(ctx) => ctx,
                None => return Ok(()),
            };
            msg = ctx.msg;
        }

        match msg.body {
            NetworkBody::Push(m) => self.face.send_push(m),
            NetworkBody::Declare(m) => self.face.send_declare(m),
            NetworkBody::Request(m) => self.face.send_request(m),
            NetworkBody::Response(m) => self.face.send_response(m),
            NetworkBody::ResponseFinal(m) => self.face.send_response_final(m),
            NetworkBody::OAM(_m) => (),
        }

        // match ctx.msg.body {
        //     NetworkBody::Declare(m) => self.face.send_declare(RoutingContext::new(m, ctx.inface)),
        //     NetworkBody::Push(m) => self.face.send_push(RoutingContext::new(m, ctx.inface)),
        //     NetworkBody::Request(m) => self.face.send_request(RoutingContext::new(m, ctx.inface)),
        //     NetworkBody::Response(m) => self.face.send_response(RoutingContext::new(m, ctx.inface)),
        //     NetworkBody::ResponseFinal(m) => self.face.send_response_final(RoutingContext::new(m, ctx.inface)),
        //     NetworkBody::OAM(_m) => (),
        // }

        Ok(())
    }

    fn new_link(&self, _link: Link) {}

    fn del_link(&self, _link: Link) {}

    fn closing(&self) {
        self.face.send_close();
        if let Some(transport) = self.transport.as_ref() {
            let ctrl_lock = zlock!(self.face.tables.ctrl_lock);
            let mut tables = zwrite!(self.face.tables.tables);
            let _ = ctrl_lock.closing(&mut tables, &self.face.tables, transport);
        }
    }

    fn closed(&self) {}

    fn as_any(&self) -> &dyn Any {
        self
    }
}

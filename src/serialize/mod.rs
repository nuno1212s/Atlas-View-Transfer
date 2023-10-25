use std::marker::PhantomData;
use std::sync::Arc;
use atlas_common::ordering::{Orderable, SeqNo};
use atlas_communication::message::Header;
use atlas_communication::reconfiguration_node::NetworkInformationProvider;
use atlas_core::ordering_protocol::networking::serialize::{OrderingProtocolMessage, PermissionedOrderingProtocolMessage, ViewTransferProtocolMessage};
use atlas_core::ordering_protocol::networking::signature_ver::OrderProtocolSignatureVerificationHelper;
use atlas_core::ordering_protocol::View;

/// The view protocol transfer message
#[derive(Clone, Debug)]
pub struct ViewTransferMessage<V> {
    sequence: SeqNo,
    view_transfer_message_kind: V,
}

/// The types of view transfer messages
#[derive(Clone, Debug)]
pub enum ViewTransferMessageKind<V> {
    RequestView,
    ViewResponse(V),
}

pub struct ViewTransfer<VT>(PhantomData<(VT)>);

impl<VT> ViewTransferProtocolMessage for ViewTransfer<VT> where VT: PermissionedOrderingProtocolMessage {
    type ProtocolMessage = ViewTransferMessage<View<VT>>;

    fn verify_view_transfer_message<NI, D, OPM, OPVH>(network_info: &Arc<NI>, header: &Header, message: Self::ProtocolMessage) -> atlas_common::error::Result<Self::ProtocolMessage> where NI: NetworkInformationProvider, D: atlas_smr_application::serialize::ApplicationData, OPM: OrderingProtocolMessage<D>, OPVH: OrderProtocolSignatureVerificationHelper<D, Self, NI>, Self: Sized {
        Ok(message)
    }
}

impl<V> ViewTransferMessage<V> {

    pub fn new(seq: SeqNo, message_kind: ViewTransferMessageKind<V>) -> Self {
        Self {
            sequence: seq,
            view_transfer_message_kind: message_kind
        }
    }

    pub fn kind(&self) -> &ViewTransferMessageKind<V> {
        &self.view_transfer_message_kind
    }

    pub fn into_kind(self) -> ViewTransferMessageKind<V> {
        self.view_transfer_message_kind
    }

}

impl<V> Orderable for ViewTransferMessage<V> {
    fn sequence_number(&self) -> SeqNo {
        self.sequence
    }
}
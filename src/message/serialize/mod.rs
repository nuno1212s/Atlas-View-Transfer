use std::marker::PhantomData;
use std::sync::Arc;
use atlas_common::ordering::{Orderable, SeqNo};
use atlas_communication::message::Header;
use atlas_communication::reconfiguration_node::NetworkInformationProvider;
use atlas_core::ordering_protocol::networking::serialize::{OrderingProtocolMessage, PermissionedOrderingProtocolMessage, ViewTransferProtocolMessage};
use atlas_core::ordering_protocol::networking::signature_ver::OrderProtocolSignatureVerificationHelper;
use atlas_core::ordering_protocol::View;
use crate::message::ViewTransferMessage;

/// View transfer type
pub struct ViewTransfer<VT>(PhantomData<fn() -> VT>);

impl<VT> ViewTransferProtocolMessage for ViewTransfer<VT>
    where VT: PermissionedOrderingProtocolMessage {
    type ProtocolMessage = ViewTransferMessage<View<VT>>;

    fn verify_view_transfer_message<NI>(network_info: &Arc<NI>,
                                        header: &Header,
                                        message: Self::ProtocolMessage) -> atlas_common::error::Result<Self::ProtocolMessage>
        where NI: NetworkInformationProvider, Self: Sized {
        Ok(message)
    }
}


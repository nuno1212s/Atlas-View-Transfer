use crate::message::ViewTransferMessage;
use atlas_common::ordering::{Orderable, SeqNo};
use atlas_communication::message::Header;
use atlas_communication::reconfiguration::NetworkInformationProvider;
use atlas_communication::serialization::InternalMessageVerifier;
use atlas_core::ordering_protocol::networking::serialize::{
    OrderingProtocolMessage, PermissionedOrderingProtocolMessage, ViewTransferProtocolMessage,
};
use atlas_core::ordering_protocol::View;
use std::marker::PhantomData;
use std::sync::Arc;

/// View transfer type
pub struct ViewTransfer<VT>(PhantomData<fn() -> VT>);

pub struct ViewTransferVerifier<VT>(PhantomData<fn() -> VT>);

impl<VT> ViewTransferProtocolMessage for ViewTransfer<VT>
where
    VT: PermissionedOrderingProtocolMessage,
{
    type ProtocolMessage = ViewTransferMessage<View<VT>>;

    fn internally_verify_message<NI>(
        network_info: &Arc<NI>,
        header: &Header,
        message: &Self::ProtocolMessage,
    ) -> atlas_common::error::Result<()>
    where
        NI: NetworkInformationProvider,
    {
        Ok(())
    }
}

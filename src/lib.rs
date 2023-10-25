use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::time::Instant;
use log::{info, warn};
use atlas_common::collections::HashMap;
use atlas_common::crypto::hash::Digest;
use atlas_common::node_id::NodeId;
use atlas_common::ordering::{Orderable, SeqNo};
use atlas_communication::message::{Header, StoredMessage};
use atlas_core::ordering_protocol::networking::serialize::PermissionedOrderingProtocolMessage;
use atlas_core::ordering_protocol::permissioned::{ViewTransferProtocol, VTMsg, VTResult, VTTimeoutResult};
use atlas_core::ordering_protocol::{PermissionedOrderingProtocol, View};
use atlas_core::ordering_protocol::networking::ViewTransferProtocolSendNode;
use atlas_core::timeouts::RqTimeout;
use atlas_metrics::metrics::metric_duration;
use crate::metrics::VIEW_TRANSFER_PROCESS_MESSAGE_TIME_ID;
use crate::serialize::{ViewTransfer, ViewTransferMessage, ViewTransferMessageKind};

mod serialize;
mod metrics;

/// The current state of the transfer protocol
pub enum TransferState<V> {
    // The transfer protocol is idling
    Idle,
    // We have requested a view
    Requested(usize, usize, HashMap<Digest, Vec<ReceivedView<V>>>),
}

/// The struct that delines the behaviour of a simple
/// view change message
pub struct SimpleViewTransferProtocol<VT, NT>
    where VT: PermissionedOrderingProtocolMessage {
    current_seq_no: SeqNo,
    // The nodes we currently know
    known_nodes: BTreeSet<NodeId>,
    // The current state of the view transfer protocol
    current_state: TransferState<View<VT>>,
    // The node type
    node: Arc<NT>,
}

impl<VT, NT> SimpleViewTransferProtocol<VT, NT>
    where VT: PermissionedOrderingProtocolMessage {
    fn next_seq(&mut self) {
        self.current_seq_no = self.current_seq_no.next()
    }
}

/// Received
#[derive(Clone, Debug)]
pub struct ReceivedView<V> {
    node: NodeId,
    digest: Digest,
    view: V,
}

enum ViewTransferResponse<V> {
    Ignored,
    NoneFound,
    ReRunProtocol,
    ViewReceived(V)
}

impl<OP, NT> ViewTransferProtocol<OP, NT> for SimpleViewTransferProtocol<OP::PermissionedSerialization, NT> where OP: PermissionedOrderingProtocol {

    type Serialization = ViewTransfer<OP::PermissionedSerialization>;
    type Config = ();

    fn initialize_view_transfer_protocol(config: Self::Config, net: Arc<NT>, view: Vec<NodeId>) -> atlas_common::error::Result<Self>
        where NT: ViewTransferProtocolSendNode<Self::Serialization> {

        let mut known_nodes = BTreeSet::new();

        for node in view {
            known_nodes.insert(node);
        }

        Ok(Self {
            current_seq_no: SeqNo::ZERO,
            known_nodes,
            current_state: TransferState::Idle,
            node: net,
        })
    }

    fn request_latest_view(&mut self, op: &OP) -> atlas_common::error::Result<()>
        where NT: ViewTransferProtocolSendNode<Self::Serialization> {
        let message = ViewTransferMessage::<View<OP::PermissionedSerialization>>::new(self.sequence_number(), ViewTransferMessageKind::RequestView);

        self.node.broadcast_signed(message, self.known_nodes.clone().into_iter())?;

        self.current_state = TransferState::Requested(OP::get_quorum_for_n(self.known_nodes.len()), 0, Default::default());

        Ok(())
    }

    fn process_message(&mut self, op: &mut OP, message: StoredMessage<VTMsg<Self::Serialization>>) -> atlas_common::error::Result<VTResult>
        where NT: ViewTransferProtocolSendNode<Self::Serialization> {
        let start = Instant::now();

        let (header, message): (Header, ViewTransferMessage<View<OP::PermissionedSerialization>>) = message.into_inner();

        let seq = message.sequence_number();

        let response = match message.into_kind() {
            ViewTransferMessageKind::RequestView => {
                let response_message = ViewTransferMessage::<View<OP::PermissionedSerialization>>::new(seq, ViewTransferMessageKind::ViewResponse(op.view()));

                self.node.send_signed(response_message, header.from(), false)?;

                ViewTransferResponse::NoneFound
            }
            ViewTransferMessageKind::ViewResponse(view) if seq == self.sequence_number() => {
                match &mut self.current_state {
                    TransferState::Requested(reqs_sent, received_views, received) => {
                        let received_states = received.entry(*header.digest());

                        let received_by_digest = received_states.or_insert_with(Vec::new);

                        received_by_digest.push(ReceivedView {
                            node: header.from(),
                            digest: header.digest().clone(),
                            view,
                        });

                        *received_views += 1;

                        let quorum = OP::get_quorum_for_n(*reqs_sent);

                        if received_by_digest.len() >= quorum {
                            let f = OP::get_f_for_n(*reqs_sent);

                            let mut received_count: Vec<_> = received.iter().map(|(digest, views)| {
                                (digest.clone(), views.len())
                            }).filter(|(digest, views)| {
                                *views > f
                            }).collect();

                            received_count
                                .sort_by(|(digest, amount), (digest_2, amount_2)| amount.cmp(amount_2).reverse());

                            if received_count.len() == 1 {
                                let (digest, count) = received_count.first().unwrap();

                                if *count >= quorum {
                                    let mut x = received.remove(digest).unwrap();

                                    Some(x.pop().unwrap().into_view())
                                } else if received_count.len() > 1 {
                                    // If we have more than one different view with more than f votes,
                                    // Then we have a problem.

                                    // Collect all nodes known in all views and re run this protocol,
                                    // Such that we have a more accurate view of the protocol status
                                    // Could we also try to take a look at the sequence number of the view?
                                    //FIXME: This could be solved by requiring quorum signatures of the
                                    // Views the nodes send, but that would increase development complexity
                                    // And we are currently striving for pace
                                    received.iter().for_each(|(digest, view)| {
                                        let view = view.first().unwrap();

                                        for node in view.view().quorum_members() {
                                            self.known_nodes.insert(node)
                                        }
                                    });

                                    ViewTransferResponse::ReRunProtocol
                                } else {
                                    warn!("Received quorum {} of views but we do not have {} matching views for any of them, {:?}", received_views, quorum, received);
                                    ViewTransferResponse::NoneFound
                                }
                            } else {
                                warn!("Received quorum {} of views but we do not have {} matching views for any of them, {:?}", received_views, f, received);
                                ViewTransferResponse::NoneFound
                            }
                        } else {
                            ViewTransferResponse::NoneFound
                        }
                    }
                    TransferState::Idle => {
                        info!("Received view message while view in idle. Message seq {:?}, header: {:?}, message: {:?}", seq, header, view);

                        ViewTransferResponse::Ignored
                    }
                }
            }
        };

        metric_duration(VIEW_TRANSFER_PROCESS_MESSAGE_TIME_ID, start.elapsed());

        match response {
            ViewTransferResponse::ViewReceived(view) => {
                op.install_view(view);

                self.current_state = TransferState::Idle;

                Ok(VTResult::VTransferFinished)
            }
            ViewTransferResponse::ReRunProtocol => {
                self.request_latest_view(op)?;

                Ok(VTResult::VTransferRunning)
            }
            ViewTransferResponse::Ignored => {
                Ok(VTResult::VTransferNotNeeded)
            }
            ViewTransferResponse::NoneFound => {
                Ok(VTResult::VTransferRunning)
            }
        }
    }

    fn handle_timeout(&mut self, timeout: Vec<RqTimeout>) -> atlas_common::error::Result<VTTimeoutResult>
        where NT: ViewTransferProtocolSendNode<Self::Serialization> {
        todo!()
    }
}

impl<VT, NT> Orderable for SimpleViewTransferProtocol<VT, NT> {
    fn sequence_number(&self) -> SeqNo {
        self.current_seq_no
    }
}

impl<V> ReceivedView<V> {
    fn into_view(self) -> V {
        self.view
    }
    pub fn view(&self) -> &V {
        &self.view
    }
}
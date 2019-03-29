use super::element_async::{BoxModel, Element, ElementQuad};
use crate::protocol::{self, dom, page};
use log::*;
use std::fmt;

#[derive(Debug)]
pub enum PageMessage {
    DocumentAvailable,
    FindNode(Option<&'static str>, Option<dom::Node>),
    FindElement(Option<&'static str>, Option<Element>),
    GetBoxModel(Option<&'static str>, dom::NodeId, BoxModel),
    Screenshot(
        Option<&'static str>,
        page::ScreenshotFormat,
        bool,
        Option<Vec<u8>>,
    ),
    MessageAvailable(protocol::Message),
    Interval,
}

// impl fmt::Debug for PageMessage {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         info!("----------------enter fmt---------------------------");
//         match self {
//             PageMessage::FindElement(selector, ele) => {
//                 let a = selector.map_or("None", |v| v);
//                 if let Some(el) = ele {
//                     write!(
//                         f,
//                         "selector: {}, remote_object_id: {}, backend_node_id: {}",
//                         a, el.remote_object_id, el.backend_node_id
//                     )
//                 } else {
//                     write!(f, "selector: {}, None", a)
//                 }
//             }
//             _ => write!(f, "{}", self),
//         }
//     }
// }

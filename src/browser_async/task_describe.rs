use super::dev_tools_method_util::SessionId;
use super::id_type as ids;
use super::page_message::{ChangingFrame, PageEventName};
use super::unique_number;
use crate::browser::tab::element::BoxModel;
use crate::browser_async::dev_tools_method_util::{
    next_call_id, ChromePageError, MethodDestination, MethodUtil,
};
use crate::protocol::{dom, page, runtime, target};
use failure;
use log::*;

#[derive(Debug)]
pub enum TaskDescribe {
    QuerySelector(QuerySelector),
    DescribeNode(Box<DescribeNode>),
    ResolveNode(ResolveNode),
    GetBoxModel(Box<GetBoxModel>),
    SetChildNodes(target::TargetId, dom::NodeId, Vec<dom::Node>),
    GetDocument(Box<GetDocument>),
    PageEnable(CommonDescribeFields),
    RuntimeEnable(CommonDescribeFields),
    Interval,
    PageEvent(PageEventName),
    FrameNavigated(target::TargetId, Box<ChangingFrame>),
    LoadEventFired(target::TargetId, f32),
    TargetInfoChanged(target::TargetInfo),
    PageCreated(target::TargetInfo, Option<&'static str>),
    PageAttached(target::TargetInfo, SessionId),
    ScreenShot(ScreenShot),
    TargetSetDiscoverTargets(bool, CommonDescribeFields),
    ChromeConnected,
    Fail,
    RuntimeEvaluate(RuntimeEvaluate),
}

impl TaskDescribe {
    pub fn get_common_fields(&self) -> Option<&CommonDescribeFields> {
        match &self {
            TaskDescribe::QuerySelector(query_selector) => Some(&query_selector.common_fields),

            TaskDescribe::DescribeNode(describe_node) => Some(&describe_node.common_fields),

            TaskDescribe::GetDocument(get_document) => Some(&get_document.common_fields),
            TaskDescribe::GetBoxModel(get_box_model) => Some(&get_box_model.common_fields),
            TaskDescribe::ScreenShot(screen_shot) => Some(&screen_shot.common_fields),
            TaskDescribe::PageEnable(common_fields)
            | TaskDescribe::TargetSetDiscoverTargets(_, common_fields)
            | TaskDescribe::RuntimeEnable(common_fields) => Some(&common_fields),
            TaskDescribe::RuntimeEvaluate(runtime_evaluate) => {
                Some(&runtime_evaluate.common_fields)
            }
            _ => {
                error!("get_common_fields got queried. but it doesn't implement that.");
                None
            }
        }
    }
}

impl std::convert::TryFrom<&TaskDescribe> for String {
    type Error = failure::Error;

    fn try_from(task_describe: &TaskDescribe) -> Result<Self, Self::Error> {
        match task_describe {
            TaskDescribe::QuerySelector(QuerySelector {
                node_id: Some(node_id_value),
                common_fields,
                selector,
                ..
            }) => Ok(MethodUtil::create_msg_to_send_with_session_id(
                dom::methods::QuerySelector {
                    node_id: *node_id_value,
                    selector,
                },
                &common_fields.session_id,
                common_fields.call_id,
            )),
            TaskDescribe::DescribeNode(describe_node) => {
                Ok(MethodUtil::create_msg_to_send_with_session_id(
                    dom::methods::DescribeNode {
                        node_id: describe_node.node_id,
                        backend_node_id: describe_node.backend_node_id,
                        depth: describe_node.depth,
                    },
                    &describe_node.common_fields.session_id,
                    describe_node.common_fields.call_id,
                ))
            }
            TaskDescribe::GetBoxModel(get_box_model) => {
                Ok(MethodUtil::create_msg_to_send_with_session_id(
                    dom::methods::GetBoxModel {
                        node_id: get_box_model.node_id,
                        backend_node_id: get_box_model.backend_node_id,
                        object_id: get_box_model.object_id.as_ref().map(Self::as_str),
                    },
                    &get_box_model.common_fields.session_id,
                    get_box_model.common_fields.call_id,
                ))
            }
            TaskDescribe::ScreenShot(ScreenShot {
                clip,
                format,
                common_fields,
                from_surface,
                ..
            }) => {
                let (format, quality) = match format {
                    page::ScreenshotFormat::JPEG(quality) => {
                        (page::InternalScreenshotFormat::JPEG, *quality)
                    }
                    page::ScreenshotFormat::PNG => (page::InternalScreenshotFormat::PNG, None),
                };
                Ok(MethodUtil::create_msg_to_send_with_session_id(
                    page::methods::CaptureScreenshot {
                        clip: clip.as_ref().cloned(),
                        format,
                        quality,
                        from_surface: *from_surface,
                    },
                    &common_fields.session_id,
                    common_fields.call_id,
                ))
            }
            TaskDescribe::GetDocument(get_document) => {
                Ok(MethodUtil::create_msg_to_send_with_session_id(
                    dom::methods::GetDocument {
                        depth: get_document.depth.or(Some(0)),
                        pierce: Some(get_document.pierce),
                    },
                    &get_document.common_fields.session_id,
                    get_document.common_fields.call_id,
                ))
            }
            TaskDescribe::PageEnable(CommonDescribeFields {
                session_id,
                call_id,
                ..
            }) => Ok(MethodUtil::create_msg_to_send_with_session_id(
                page::methods::Enable {},
                session_id,
                *call_id,
            )),
            TaskDescribe::RuntimeEnable(common_fields) => Ok(MethodUtil::create_msg_to_send(
                runtime::methods::Enable {},
                MethodDestination::Browser,
                common_fields.call_id,
            )),
            TaskDescribe::TargetSetDiscoverTargets(enable, common_fields) => {
                Ok(MethodUtil::create_msg_to_send(
                    target::methods::SetDiscoverTargets { discover: *enable },
                    MethodDestination::Browser,
                    common_fields.call_id,
                ))
            }
            TaskDescribe::RuntimeEvaluate(runtime_evaluate) => {
                Ok(MethodUtil::create_msg_to_send_with_session_id(
                    runtime::methods::Evaluate {
                        expression: runtime_evaluate.expression.as_str(),
                        object_group: runtime_evaluate.object_group.as_ref().map(Self::as_str),
                        include_command_line_a_p_i: runtime_evaluate.include_command_line_a_p_i,
                        silent: runtime_evaluate.silent,
                        context_id: runtime_evaluate.context_id,
                        return_by_value: runtime_evaluate.return_by_value,
                        generate_preview: runtime_evaluate.generate_preview,
                        user_gesture: runtime_evaluate.user_gesture,
                        await_promise: runtime_evaluate.await_promise,
                        throw_on_side_effect: runtime_evaluate.throw_on_side_effect,
                        time_out: runtime_evaluate.time_out,
                    },
                    &runtime_evaluate.common_fields.session_id,
                    runtime_evaluate.common_fields.call_id,
                ))
            }
            _ => {
                error!("task describe to string failed. {:?}", task_describe);
                Err(ChromePageError::TaskDescribeConvert.into())
            }
        }
    }
}

#[derive(Debug, Builder, Clone)]
#[builder(setter(into))]
pub struct RuntimeEvaluate {
    pub common_fields: CommonDescribeFields,
    pub expression: String,
    #[builder(default = "None")]
    pub object_group: Option<String>,
    #[builder(default = "None")]
    pub include_command_line_a_p_i: Option<bool>,
    #[builder(default = "None")]
    pub silent: Option<bool>,
    #[builder(default = "None")]
    pub context_id: Option<runtime::types::ExecutionContextId>,
    #[builder(default = "None")]
    pub return_by_value: Option<bool>,
    #[builder(default = "None")]
    pub generate_preview: Option<bool>,
    #[builder(default = "None")]
    pub user_gesture: Option<bool>,
    #[builder(default = "None")]
    pub await_promise: Option<bool>,
    #[builder(default = "None")]
    pub throw_on_side_effect: Option<bool>,
    #[builder(default = "None")]
    pub time_out: Option<runtime::types::TimeDelta>,
}

impl From<RuntimeEvaluate> for TaskDescribe {
    fn from(runtime_evaluate: RuntimeEvaluate) -> Self {
        TaskDescribe::RuntimeEvaluate(runtime_evaluate)
    }
}

#[derive(Debug, Builder, Clone)]
#[builder(setter(into))]
pub struct ScreenShot {
    pub common_fields: CommonDescribeFields,
    pub selector: Option<&'static str>,
    pub format: page::ScreenshotFormat,
    #[builder(default = "None")]
    pub clip: Option<page::Viewport>,
    #[builder(default = "false")]
    pub from_surface: bool,
    #[builder(default = "None")]
    pub base64: Option<String>,
}

impl From<ScreenShot> for TaskDescribe {
    fn from(screen_shot: ScreenShot) -> Self {
        TaskDescribe::ScreenShot(screen_shot)
    }
}

#[derive(Debug, Builder, Default)]
#[builder(setter(into))]
pub struct GetBoxModel {
    pub common_fields: CommonDescribeFields,
    #[builder(default = "None")]
    pub node_id: Option<dom::NodeId>,
    #[builder(default = "None")]
    pub selector: Option<&'static str>,
    #[builder(default = "None")]
    pub backend_node_id: Option<dom::NodeId>,
    #[builder(default = "None")]
    pub object_id: Option<ids::RemoteObject>,
    #[builder(setter(skip))]
    pub found_box: Option<BoxModel>,
}

impl From<GetBoxModel> for TaskDescribe {
    fn from(get_box_model: GetBoxModel) -> Self {
        TaskDescribe::GetBoxModel(Box::new(get_box_model))
    }
}

#[derive(Debug)]
pub struct ResolveNode {
    pub common_fields: CommonDescribeFields,
    pub selector: Option<&'static str>,
    pub node_id: Option<dom::NodeId>,
    pub backend_node_id: Option<dom::NodeId>,
    pub object_group: Option<String>,
    pub execution_context_id: Option<String>,
}

#[derive(Debug, Builder, Default)]
#[builder(setter(into))]
pub struct QuerySelector {
    pub common_fields: CommonDescribeFields,
    #[builder(default = "None")]
    pub node_id: Option<dom::NodeId>,
    #[builder(default = "None")]
    pub found_node_id: Option<dom::NodeId>,
    pub selector: &'static str,
}

impl From<QuerySelector> for TaskDescribe {
    fn from(query_selector: QuerySelector) -> Self {
        TaskDescribe::QuerySelector(query_selector)
    }
}

#[derive(Debug, Builder, Default)]
#[builder(setter(into))]
pub struct GetDocument {
    pub common_fields: CommonDescribeFields,
    #[builder(default = "Some(0)")]
    pub depth: Option<u8>,
    #[builder(default = "false")]
    pub pierce: bool,
    #[builder(setter(skip))]
    pub root_node: Option<dom::Node>,
}

impl From<GetDocument> for TaskDescribe {
    fn from(get_document: GetDocument) -> Self {
        TaskDescribe::GetDocument(Box::new(get_document))
    }
}

#[derive(Debug, Builder, Default)]
#[builder(setter(into))]
pub struct DescribeNode {
    pub common_fields: CommonDescribeFields,
    #[builder(default = "None")]
    pub node_id: Option<dom::NodeId>,
    #[builder(default = "None")]
    pub backend_node_id: Option<dom::NodeId>,
    #[builder(default = "None")]
    pub found_node: Option<dom::Node>,
    pub selector: Option<&'static str>,
    #[builder(default = "Some(0)")]
    pub depth: Option<i8>,
    #[builder(default = "None")]
    pub object_id: Option<ids::RemoteObject>,
    #[builder(default = "false")]
    pub pierce: bool,
}

impl From<DescribeNode> for TaskDescribe {
    fn from(describe_node: DescribeNode) -> Self {
        TaskDescribe::DescribeNode(Box::new(describe_node))
    }
}

#[derive(Debug, Clone, Default, Builder)]
#[builder(setter(into))]
pub struct CommonDescribeFields {
    #[builder(default = "None")]
    pub target_id: Option<target::TargetId>,
    #[builder(default = "None")]
    pub session_id: Option<SessionId>,
    #[builder(default = "unique_number::create_one()")]
    #[builder(setter(prefix = "_abc"))]
    pub task_id: ids::Task,
    #[builder(default = "next_call_id()")]
    pub call_id: usize,
}

impl CommonDescribeFieldsBuilder {
    pub fn task_id(&mut self, task_id: impl Into<Option<ids::Task>>) -> &mut Self {
        let o = task_id.into();
        if o.is_some() {
            self.task_id = o;
        }
        self
    }
}

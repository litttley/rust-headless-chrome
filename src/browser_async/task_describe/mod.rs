use super::inner_event::inner_events;
use crate::browser_async::{
    create_msg_to_send, create_msg_to_send_with_session_id, next_call_id, ChromePageError,
    CreateMethodCallString, MethodDestination, TaskId, create_unique_usize
};
use crate::protocol::{dom, page, runtime, target};
use failure;
use log::*;

pub mod dom_task;
pub mod page_task;
pub mod runtime_task;
pub mod target_task;

pub use dom_task::{
    describe_node::DescribeNodeTask, describe_node::DescribeNodeTaskBuilder,
    get_box_model::GetBoxModelTask, get_box_model::GetBoxModelTaskBuilder,
    get_document::GetDocumentTask, get_document::GetDocumentTaskBuilder,
    query_selector::QuerySelectorTask, query_selector::QuerySelectorTaskBuilder,
};
pub use page_task::{
    capture_screenshot::CaptureScreenshotTask, capture_screenshot::CaptureScreenshotTaskBuilder,
    navigate_to::NavigateToTask, navigate_to::NavigateToTaskBuilder, page_enable::PageEnableTask,
    print_to_pdf::PrintToPdfTask, print_to_pdf::PrintToPdfTaskBuilder,
};
pub use runtime_task::{
    call_function_on::RuntimeCallFunctionOnTask,
    call_function_on::RuntimeCallFunctionOnTaskBuilder, evaluate::RuntimeEvaluateTask,
    evaluate::RuntimeEvaluateTaskBuilder, get_properties::RuntimeGetPropertiesTask,
    get_properties::RuntimeGetPropertiesTaskBuilder,
};

pub use target_task::{
    create_target::CreateTargetTask, create_target::CreateTargetTaskBuilder,
};

#[derive(Debug)]
pub enum TaskDescribe {
    NavigateTo(Box<NavigateToTask>),
    QuerySelector(QuerySelectorTask),
    DescribeNode(Box<DescribeNodeTask>),
    ResolveNode(ResolveNode),
    PrintToPDF(Box<PrintToPdfTask>),
    GetBoxModel(Box<GetBoxModelTask>),
    SetChildNodes(target::TargetId, dom::NodeId, Vec<dom::Node>),
    GetDocument(Box<GetDocumentTask>),
    PageEnable(PageEnableTask),
    RuntimeEnable(CommonDescribeFields),
    Interval,
    FrameAttached(page::events::FrameAttachedParams, CommonDescribeFields),
    FrameDetached(page::types::FrameId, CommonDescribeFields),
    FrameStartedLoading(String, CommonDescribeFields),
    FrameNavigated(Box<page::Frame>, CommonDescribeFields),
    FrameStoppedLoading(String, CommonDescribeFields),
    LoadEventFired(target::TargetId, f32),
    TargetInfoChanged(target::TargetInfo, CommonDescribeFields),
    PageCreated(target::TargetInfo, Option<String>),
    PageAttached(target::TargetInfo, target::SessionID),
    CaptureScreenshot(Box<CaptureScreenshotTask>),
    TargetSetDiscoverTargets(bool, CommonDescribeFields),
    ChromeConnected,
    Fail,
    RuntimeEvaluate(Box<RuntimeEvaluateTask>),
    RuntimeGetProperties(Box<RuntimeGetPropertiesTask>),
    RuntimeExecutionContextCreated(
        runtime::types::ExecutionContextDescription,
        CommonDescribeFields,
    ),
    RuntimeExecutionContextDestroyed(runtime::types::ExecutionContextId, CommonDescribeFields),
    RuntimeConsoleAPICalled(inner_events::ConsoleAPICalledParams, CommonDescribeFields),
    RuntimeCallFunctionOn(Box<RuntimeCallFunctionOnTask>),
    CreateTarget(Box<CreateTargetTask>),
}

impl TaskDescribe {
    pub fn get_common_fields(&self) -> Option<&CommonDescribeFields> {
        let common_fields = CommonDescribeFieldsBuilder::default().build().unwrap();
        match &self {
            TaskDescribe::QuerySelector(query_selector) => Some(&query_selector.common_fields),
            TaskDescribe::DescribeNode(describe_node) => Some(&describe_node.common_fields),
            TaskDescribe::GetDocument(get_document) => Some(&get_document.common_fields),
            TaskDescribe::GetBoxModel(get_box_model) => Some(&get_box_model.common_fields),
            TaskDescribe::CaptureScreenshot(screen_shot) => Some(&screen_shot.common_fields),
            TaskDescribe::NavigateTo(navigate_to) => Some(&navigate_to.common_fields),
            TaskDescribe::PrintToPDF(print_to_pdf) => Some(&print_to_pdf.common_fields),
            TaskDescribe::PageEnable(page_enable) => Some(&page_enable.common_fields),
            TaskDescribe::TargetSetDiscoverTargets(_, common_fields)
            | TaskDescribe::RuntimeEnable(common_fields) => Some(&common_fields),
            TaskDescribe::RuntimeEvaluate(runtime_evaluate) => {
                Some(&runtime_evaluate.common_fields)
            }
            TaskDescribe::RuntimeGetProperties(get_properties) => {
                Some(&get_properties.common_fields)
            }
            TaskDescribe::RuntimeCallFunctionOn(call_function_on) => {
                Some(&call_function_on.common_fields)
            }
            TaskDescribe::CreateTarget(_task) => {
                None
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
        let (session_id, call_id) = if let Some(common_fields) = &task_describe.get_common_fields() {
            (common_fields.session_id.as_ref(), common_fields.call_id)
        } else {
            (None, next_call_id())
        };
        match task_describe {
            TaskDescribe::QuerySelector(query_selector) => {
                Ok(query_selector.create_method_call_string(session_id, call_id))
            }
            TaskDescribe::DescribeNode(describe_node) => {
                Ok(describe_node.create_method_call_string(session_id, call_id))
            }
            TaskDescribe::PrintToPDF(print_to_pdf) => {
                Ok(print_to_pdf.create_method_call_string(session_id, call_id))
            }
            TaskDescribe::GetBoxModel(get_box_model) => {
                Ok(get_box_model.create_method_call_string(session_id, call_id))
            }
            TaskDescribe::CaptureScreenshot(capture_screenshot) => {
                Ok(capture_screenshot.create_method_call_string(session_id, call_id))
            }
            TaskDescribe::GetDocument(get_document) => {
                Ok(get_document.create_method_call_string(session_id, call_id))
            }
            TaskDescribe::NavigateTo(navigate_to) => {
                Ok(navigate_to.create_method_call_string(session_id, call_id))
            }
            TaskDescribe::PageEnable(page_enable) => {
                Ok(page_enable.create_method_call_string(session_id, call_id))
            }
            TaskDescribe::RuntimeEnable(common_fields) => Ok(create_msg_to_send_with_session_id(
                runtime::methods::Enable {},
                common_fields.session_id.as_ref(),
                common_fields.call_id,
            )),
            TaskDescribe::TargetSetDiscoverTargets(enable, common_fields) => {
                Ok(create_msg_to_send(
                    target::methods::SetDiscoverTargets { discover: *enable },
                    MethodDestination::Browser,
                    common_fields.call_id,
                ))
            }
            TaskDescribe::CreateTarget(create_target) => {
                Ok(create_target.create_method_call_string(session_id, call_id))
            }
            TaskDescribe::RuntimeEvaluate(runtime_evaluate) => {
                Ok(runtime_evaluate.create_method_call_string(session_id, call_id))
            }
            TaskDescribe::RuntimeGetProperties(get_properties) => {
                Ok(get_properties.create_method_call_string(session_id, call_id))
            }
            TaskDescribe::RuntimeCallFunctionOn(call_function_on) => {
                Ok(call_function_on.create_method_call_string(session_id, call_id))
            }
            _ => {
                error!("task describe to string failed. {:?}", task_describe);
                Err(ChromePageError::TaskDescribeConvert.into())
            }
        }
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

#[derive(Debug, Clone, Default, Builder)]
#[builder(setter(into))]
pub struct CommonDescribeFields {
    #[builder(default = "None")]
    pub target_id: Option<target::TargetId>,
    #[builder(default = "None")]
    pub session_id: Option<target::SessionID>,
    #[builder(default = "create_unique_usize()")]
    #[builder(setter(prefix = "_abc"))]
    pub task_id: TaskId,
    #[builder(default = "next_call_id()")]
    pub call_id: usize,
}

impl From<(Option<String>, Option<String>)> for CommonDescribeFields {
    fn from(session_id_target_id: (Option<String>, Option<String>)) -> Self {
        CommonDescribeFieldsBuilder::default()
            .target_id(session_id_target_id.1)
            .session_id(session_id_target_id.0.map(Into::into))
            .build()
            .unwrap()
    }
}

impl CommonDescribeFieldsBuilder {
    pub fn task_id(&mut self, task_id: impl Into<Option<TaskId>>) -> &mut Self {
        let o = task_id.into();
        if o.is_some() {
            self.task_id = o;
        }
        self
    }
}

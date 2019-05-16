use super::chrome_browser::ChromeBrowser;
use super::chrome_debug_session::ChromeDebugSession;
use super::interval_page_message::IntervalPageMessage;
use super::page_message::{response_object, PageResponse, PageResponseWithTargetIdTaskId};
use super::tab::Tab;
use super::task_describe::{self as tasks, TaskDescribe, CommonDescribeFields};

use crate::browser_async::{ChromePageError, TaskId,};
use std::convert::TryInto;
use crate::protocol::target;
use failure;
use futures::{Async, Poll};
use log::*;
use std::default::Default;
use std::sync::{Arc, Mutex};
use websocket::futures::Stream;

const DEFAULT_TAB_NAME: &str = "_default_tab_";

struct Wrapper {
    pub chrome_debug_session: Arc<Mutex<ChromeDebugSession>>,
}

impl Stream for Wrapper {
    type Item = TaskDescribe;
    type Error = failure::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        self.chrome_debug_session.lock().unwrap().poll()
    }
}

/// An adapter for merging the output of two streams.
///
/// The merged stream produces items from either of the underlying streams as
/// they become available, and the streams are polled in a round-robin fashion.
/// Errors, however, are not merged: you get at most one error at a time.
// #[derive(Debug)]
#[must_use = "streams do nothing unless polled"]
pub struct DebugSession {
    interval_page_message: IntervalPageMessage,
    pub chrome_debug_session: Arc<Mutex<ChromeDebugSession>>,
    seconds_from_start: usize,
    flag: bool,
    tabs: Vec<Tab>, // early created at front.
    wrapper: Wrapper,
}

impl Default for DebugSession {
    fn default() -> Self {
        let browser = ChromeBrowser::new();
        let chrome_debug_session = ChromeDebugSession::new(browser);
        Self::new(chrome_debug_session)
    }
}

impl DebugSession {
    pub fn new(chrome_debug_session: ChromeDebugSession) -> Self {
        let interval_page_message = IntervalPageMessage::new();
        let arc_cds = Arc::new(Mutex::new(chrome_debug_session));
        Self {
            interval_page_message,
            chrome_debug_session: arc_cds.clone(),
            seconds_from_start: 0,
            flag: false,
            tabs: Vec::new(),
            wrapper: Wrapper {
                chrome_debug_session: arc_cds,
            },
        }
    }
    pub fn get_tab_by_id_mut(
        &mut self,
        target_id: Option<&target::TargetId>,
    ) -> Result<&mut Tab, failure::Error> {
        if let Some(tab) = self
            .tabs.iter_mut()
            .find(|t| Some(&t.target_info.target_id) == target_id)
        {
            Ok(tab)
        } else {
            Err(ChromePageError::TabNotFound.into())
        }
    }

    pub fn create_new_tab(&mut self, url: &str) {
        let task = tasks::CreateTargetTaskBuilder::default().url(url.to_owned()).build().unwrap();
        let method_str: String = (&tasks::TaskDescribe::from(task)).try_into().expect("should convert from CreateTargetTask");
        self.chrome_debug_session
            .lock()
            .unwrap()
            .send_message_direct(method_str);
    }

    pub fn get_browser_context_ids(&self) -> Vec<&target::BrowserContextID> {
        let mut ids: Vec<&target::BrowserContextID> = self
            .tabs
            .iter()
            .filter_map(|tab| tab.target_info.browser_context_id.as_ref())
            .collect();
        ids.sort_unstable();
        ids.dedup();
        ids
    }

    pub fn get_tab_by_id(
        &self,
        target_id: Option<&target::TargetId>,
    ) -> Result<&Tab, failure::Error> {
        if let Some(tab) = self
            .tabs
            .iter()
            .find(|t| Some(&t.target_info.target_id) == target_id)
        {
            Ok(tab)
        } else {
            Err(ChromePageError::TabNotFound.into())
        }
    }

    pub fn first_page_mut(&mut self) -> Option<&mut Tab> {
        self.tabs.get_mut(0)
    }
    pub fn main_tab(&self) -> Option<&Tab> {
        self.tabs.get(0)
    }

    fn send_fail(
        &mut self,
        target_id: Option<target::TargetId>,
        task_id: Option<TaskId>,
    ) -> Poll<Option<PageResponseWithTargetIdTaskId>, failure::Error> {
        let pr = (target_id, task_id, PageResponse::Fail);
        Ok(Some(pr).into())
    }

    pub fn runtime_enable(&mut self) {
        let cf = tasks::CommonDescribeFieldsBuilder::default()
            .build()
            .unwrap();
        self.chrome_debug_session
            .lock()
            .unwrap()
            .execute_task(vec![TaskDescribe::RuntimeEnable(cf)]);
    }

    pub fn set_discover_targets(&mut self, enable: bool) {
        let cf = tasks::CommonDescribeFieldsBuilder::default()
            .build()
            .unwrap();
        self.chrome_debug_session
            .lock()
            .unwrap()
            .execute_task(vec![TaskDescribe::TargetSetDiscoverTargets(enable, cf)]);
    }

    fn send_fail_1(
        &mut self,
        common_fields: Option<&CommonDescribeFields>,
    ) -> Poll<Option<PageResponseWithTargetIdTaskId>, failure::Error> {
        if let Some(cf) = common_fields {
            let pr = (cf.target_id.clone(), Some(cf.task_id), PageResponse::Fail);
            Ok(Some(pr).into())
        } else {
            let pr = (None, None, PageResponse::Fail);
            Ok(Some(pr).into())
        }
    }

    fn convert_to_page_response(
        &self,
        common_fields: Option<&CommonDescribeFields>,
        page_response: PageResponse,
    ) -> Option<PageResponseWithTargetIdTaskId> {
        trace!("got page response: {:?}", page_response);
        if let Some(cf) = common_fields {
            Some((cf.target_id.clone(), Some(cf.task_id), page_response))
        } else {
            Some((None, None, page_response))
        }
    }

    pub fn send_page_message(
        &mut self,
        item: TaskDescribe,
    ) -> Poll<Option<PageResponseWithTargetIdTaskId>, failure::Error> {
        match item {
            TaskDescribe::Interval => {
                self.seconds_from_start += 1;
                let pr = (
                    None,
                    None,
                    PageResponse::SecondsElapsed(self.seconds_from_start),
                );
                Ok(Some(pr).into())
            }
            TaskDescribe::PageCreated(target_info, page_name) => {
                info!(
                    "receive page created event: {:?}, page_name: {:?}",
                    target_info, page_name
                );
                let target_id = target_info.target_id.clone();
                let tab = Tab::new(target_info, Arc::clone(&self.chrome_debug_session));
                self.tabs.push(tab);
                let idx = self.tabs.len();
                let pr = (
                    Some(target_id),
                    None,
                    PageResponse::PageCreated(idx),
                );
                Ok(Some(pr).into())
            }
            TaskDescribe::PageAttached(target_info, session_id) => {
                // each attach return different session_id.
                // when the chrome process started, it default creates a target and attach it. the url is: about:blank
                info!(
                    "receive page attached event: {:?}, session_id: {:?}",
                    target_info,
                    session_id.clone()
                );
                match self.get_tab_by_id_mut(Some(&target_info.target_id)) {
                    Ok(tab) => {
                        tab.session_id.replace(session_id.clone());
                        // tab.page_enable();
                        let pr = (
                            Some(target_info.target_id.clone()),
                            None,
                            PageResponse::PageAttached(target_info, session_id),
                        );
                        Ok(Some(pr).into())
                    }
                    Err(error) => {
                        error!("page attached event has caught, but cannot find corresponding tab. {:?}", error);
                        self.send_fail(None, None)
                    }
                }
            }
            TaskDescribe::PageEnable(page_enable) => {
                info!("page_enabled: {:?}", page_enable);
                let resp = self.convert_to_page_response(
                    Some(&page_enable.common_fields),
                    PageResponse::PageEnable,
                );
                Ok(resp.into())
            }
            // attached may not invoke, if invoked it's the first. then started, navigated, stopped.
            TaskDescribe::FrameNavigated(frame, common_fields) => {
                info!(
                    "-----------------frame_navigated-----------------{:?}",
                    frame
                );
                let tab = self.get_tab_by_id_mut(common_fields.target_id.as_ref())?;
                let frame_id = frame.id.clone();
                tab._frame_navigated(*frame);
                let resp = self.convert_to_page_response(
                    Some(&common_fields),
                    PageResponse::FrameNavigated(frame_id),
                );
                Ok(resp.into())
            }
            TaskDescribe::FrameStartedLoading(frame_id, common_fields) => {
                // started loading is first, then attached.
                info!(
                    "-----------------frame_started_loading-----------------{:?}",
                    frame_id
                );
                let tab = self.get_tab_by_id_mut(common_fields.target_id.as_ref())?;
                tab._frame_started_loading(frame_id.clone());
                let resp = self.convert_to_page_response(
                    Some(&common_fields),
                    PageResponse::FrameStartedLoading(frame_id),
                );
                Ok(resp.into())
            }
            TaskDescribe::FrameStoppedLoading(frame_id, common_fields) => {
                info!(
                    "-----------------frame_stopped_loading-----------------{:?}",
                    frame_id
                );
                let tab = self.get_tab_by_id_mut(common_fields.target_id.as_ref())?;
                tab._frame_stopped_loading(frame_id.clone());
                let pr = (
                    common_fields.target_id,
                    None,
                    PageResponse::FrameStoppedLoading(frame_id),
                );
                Ok(Some(pr).into())
            }
            TaskDescribe::FrameAttached(frame_attached_params, common_fields) => {
                info!(
                    "-----------------frame_attached-----------------{:?}",
                    frame_attached_params.frame_id
                );
                let tab = self.get_tab_by_id_mut(common_fields.target_id.as_ref())?;
                let frame_id = frame_attached_params.frame_id.clone();
                tab._frame_attached(frame_attached_params);
                let resp = self.convert_to_page_response(
                    Some(&common_fields),
                    PageResponse::FrameAttached(frame_id),
                );
                Ok(resp.into())
            }
            TaskDescribe::FrameDetached(frame_id, common_fields) => {
                info!(
                    "-----------------frame_detached-----------------{:?}",
                    frame_id.clone()
                );
                let tab = self.get_tab_by_id_mut(common_fields.target_id.as_ref())?;
                tab._frame_detached(&frame_id);
                self.send_fail(None, None)
            }
            TaskDescribe::GetDocument(get_document) => {
                let common_fields = &get_document.common_fields;
                let tab = self.get_tab_by_id_mut(common_fields.target_id.as_ref())?;

                tab.root_node = get_document.task_result;
                let resp =
                    self.convert_to_page_response(Some(common_fields), PageResponse::GetDocument);
                Ok(resp.into())
            }
            TaskDescribe::SetChildNodes(target_id, parent_node_id, nodes) => {
                let tab = self.get_tab_by_id_mut(Some(&target_id))?;
                tab.node_arrived(parent_node_id, nodes);
                let pr = (
                    Some(target_id),
                    None,
                    PageResponse::SetChildNodes(parent_node_id, vec![]),
                );
                Ok(Some(pr).into())
            }
            TaskDescribe::QuerySelector(query_selector) => {
                let pr = PageResponse::QuerySelector(
                    query_selector.selector,
                    query_selector.task_result,
                );
                let resp = self.convert_to_page_response(Some(&query_selector.common_fields), pr);
                Ok(resp.into())
            }
            TaskDescribe::DescribeNode(describe_node) => {
                let common_fields = &describe_node.common_fields;
                let node_id = describe_node
                    .task_result
                    .as_ref()
                    .and_then(|n| Some(n.node_id));
                let tab = self.get_tab_by_id_mut(common_fields.target_id.as_ref())?;

                tab.node_returned(describe_node.task_result);
                let resp = self.convert_to_page_response(
                    Some(&common_fields),
                    PageResponse::DescribeNode(describe_node.selector, node_id),
                );
                Ok(resp.into())
            }
            TaskDescribe::GetBoxModel(get_box_model) => {
                let common_fields = &get_box_model.common_fields;
                let resp = self.convert_to_page_response(
                    Some(&common_fields),
                    PageResponse::GetBoxModel(
                        get_box_model.selector,
                        get_box_model.task_result.map(Box::new),
                    ),
                );
                Ok(resp.into())
            }
            TaskDescribe::LoadEventFired(target_id, timestamp) => {
                let pr = (
                    Some(target_id),
                    None,
                    PageResponse::LoadEventFired(timestamp),
                );
                Ok(Some(pr).into())
            }
            TaskDescribe::CaptureScreenshot(screen_shot) => {
                let common_fields = &screen_shot.common_fields;
                let ro = response_object::CaptureScreenshot {
                    selector: screen_shot.selector,
                    base64: screen_shot.task_result,
                };
                let resp = self.convert_to_page_response(
                    Some(&common_fields),
                    PageResponse::CaptureScreenshot(ro),
                );
                Ok(resp.into())
            }
            TaskDescribe::RuntimeEvaluate(runtime_evaluate) => {
                let common_fields = &runtime_evaluate.common_fields;
                let resp = self.convert_to_page_response(
                    Some(&common_fields),
                    PageResponse::RuntimeEvaluate(
                        runtime_evaluate.task_result.map(Box::new),
                        runtime_evaluate.exception_details.map(Box::new),
                    ),
                );
                Ok(resp.into())
            }
            TaskDescribe::ChromeConnected => {
                let resp = Some((None, None, PageResponse::ChromeConnected));
                Ok(resp.into())
            }
            TaskDescribe::RuntimeEnable(common_fields) => {
                let resp = self
                    .convert_to_page_response(Some(&common_fields), PageResponse::RuntimeEnable);
                Ok(resp.into())
            }
            TaskDescribe::RuntimeExecutionContextCreated(
                runtime_execution_context_created,
                common_fields,
            ) => {
                let tab = self.get_tab_by_id_mut(common_fields.target_id.as_ref())?;
                let frame_id =
                    tab.runtime_execution_context_created(runtime_execution_context_created);
                let resp = self.convert_to_page_response(
                    Some(&common_fields),
                    PageResponse::RuntimeExecutionContextCreated(frame_id),
                );
                Ok(resp.into())
            }
            TaskDescribe::RuntimeExecutionContextDestroyed(
                runtime_execution_context_destroyed,
                common_fields,
            ) => {
                let tab = self.get_tab_by_id_mut(common_fields.target_id.as_ref())?;
                tab.runtime_execution_context_destroyed(runtime_execution_context_destroyed);
                self.send_fail_1(Some(&common_fields))
            }
            TaskDescribe::RuntimeConsoleAPICalled(console_api_called, common_fields) => {
                let tab = self.get_tab_by_id_mut(common_fields.target_id.as_ref())?;
                // let execution_context_id = console_api_called.execution_context_id.clone();
                tab.verify_execution_context_id(&console_api_called);
                self.send_fail(None, None)
            }
            TaskDescribe::TargetSetDiscoverTargets(value, _common_fields) => {
                assert!(value);
                self.send_fail(None, None)
            }
            TaskDescribe::TargetInfoChanged(target_info, common_fields) => {
                if let Ok(tab) = self.get_tab_by_id_mut(Some(&target_info.target_id)) {
                    tab.target_info = target_info;
                    trace!(
                        "target info changed: {:?}, {:?}",
                        tab.target_info,
                        common_fields
                    );
                } else {
                    warn!(
                        "target changed, no correspond tab. {:?}, {:?}",
                        target_info, common_fields
                    );
                }
                self.send_fail(None, None)
            }
            TaskDescribe::NavigateTo(navigate_to) => {
                trace!("navigate_to: {:?}", navigate_to);
                self.send_fail(None, None)
            }
            TaskDescribe::RuntimeGetProperties(get_properties) => {
                let resp = self.convert_to_page_response(
                    Some(&get_properties.common_fields),
                    PageResponse::RuntimeGetProperties(get_properties.task_result),
                );
                Ok(resp.into())
            }
            TaskDescribe::RuntimeCallFunctionOn(task) => {
                let resp = self.convert_to_page_response(
                    Some(&task.common_fields),
                    PageResponse::RuntimeCallFunctionOn(task.task_result),
                );
                Ok(resp.into())
            }
            TaskDescribe::PrintToPDF(task) => {
                let resp = self.convert_to_page_response(
                    Some(&task.common_fields),
                    PageResponse::PrintToPDF(task.task_result),
                );
                Ok(resp.into())
            }
            _ => {
                warn!("debug_session got unknown task. {:?}", item);
                self.send_fail(None, None)
            }
        }
    }
}

impl Stream for DebugSession {
    type Item = PageResponseWithTargetIdTaskId;
    type Error = failure::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        let (a, b) = if self.flag {
            (
                &mut self.wrapper as &mut Stream<Item = _, Error = _>,
                &mut self.interval_page_message as &mut Stream<Item = _, Error = _>,
            )
        } else {
            (
                &mut self.interval_page_message as &mut Stream<Item = _, Error = _>,
                &mut self.wrapper as &mut Stream<Item = _, Error = _>,
            )
        };
        self.flag = !self.flag;
        let a_done = match a.poll()? {
            Async::Ready(Some(item)) => return self.send_page_message(item),
            Async::Ready(None) => true,
            Async::NotReady => false,
        };

        match b.poll()? {
            Async::Ready(Some(item)) => {
                // If the other stream isn't finished yet, give them a chance to
                // go first next time as we pulled something off `b`.
                if !a_done {
                    self.flag = !self.flag;
                }
                self.send_page_message(item)
            }
            Async::Ready(None) if a_done => Ok(None.into()),
            Async::Ready(None) | Async::NotReady => Ok(Async::NotReady),
        }
    }
}

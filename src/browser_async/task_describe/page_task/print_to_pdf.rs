use super::super::{TaskDescribe, CommonDescribeFields, CreateMethodCallString, create_msg_to_send_with_session_id};
use crate::protocol::{page};
use crate::browser::transport::{SessionId};

#[derive(Debug, Builder, Clone)]
#[builder(setter(into))]
pub struct PrintToPdfTask {
    pub common_fields: CommonDescribeFields,
    #[builder(default = "None")]
    pub landscape: Option<bool>,
    #[builder(default = "None")]
    pub display_header_footer: Option<bool>,
    #[builder(default = "None")]
    pub print_background: Option<bool>,
    #[builder(default = "None")]
    pub scale: Option<f32>,
    #[builder(default = "None")]
    pub paper_width: Option<f32>,
    #[builder(default = "None")]
    pub paper_height: Option<f32>,
    #[builder(default = "None")]
    pub margin_top: Option<f32>,
    #[builder(default = "None")]
    pub margin_bottom: Option<f32>,
    #[builder(default = "None")]
    pub margin_left: Option<f32>,
    #[builder(default = "None")]
    pub margin_right: Option<f32>,
    #[builder(default = "None")]
    pub page_ranges: Option<String>,
    #[builder(default = "None")]
    pub ignore_invalid_page_ranges: Option<bool>,
    #[builder(default = "None")]
    pub header_template: Option<String>,
    #[builder(default = "None")]
    pub footer_template: Option<String>,
    #[builder(default = "None")]
    pub prefer_css_page_size: Option<bool>,
    #[builder(default = "None")]
    pub result: Option<String>,
}

impl From<PrintToPdfTask> for TaskDescribe {
    fn from(print_to_pdf: PrintToPdfTask) -> Self {
        TaskDescribe::PrintToPDF(Box::new(print_to_pdf))
    }
}

impl CreateMethodCallString for PrintToPdfTask {
    fn create_method_call_string(&self, session_id: Option<&SessionId>, call_id: usize) -> String {
                let options = Some(page::PrintToPdfOptions {
            landscape: self.landscape,
            display_header_footer: self.display_header_footer,
            print_background: self.print_background,
            scale: self.scale,
            paper_width: self.paper_width,
            paper_height: self.paper_height,
            margin_top: self.margin_top,
            margin_bottom: self.margin_bottom,
            margin_right: self.margin_right,
            margin_left: self.margin_left,
            page_ranges: self.page_ranges.clone(),
            ignore_invalid_page_ranges: self.ignore_invalid_page_ranges,
            header_template: self.header_template.clone(),
            footer_template: self.footer_template.clone(),
            prefer_css_page_size: self.prefer_css_page_size,
        });

        let method = page::methods::PrintToPdf {
            options,
        };
                create_msg_to_send_with_session_id(
                    method,
                    session_id,
                    call_id,
                )

    }
}
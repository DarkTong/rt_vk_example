use ash::{vk, util::*};
use std::boxed::Box;
pub struct PipelineStateObject {
    pub vs_desc: vk::PipelineShaderStageCreateInfo,
    pub ps_desc: vk::PipelineShaderStageCreateInfo,
    pub pipeline: Option<vk::Pipeline>,
    pub attachment_desc: Vec<vk::AttachmentDescription>,
    pub render_pass: Option<vk::RenderPass>,
    pub input_layout_desc: Vec<vk::VertexInputAttributeDescription>,
    pub viewport: vk::Rect2D,
    pub scissor: vk::Rect2D,

    vs_mod: Option<vk::ShaderModule>,
    ps_mod: Option<vk::ShaderModule>,
}

impl ::std::default::Default for PipelineStateObject {
    fn default() -> Self {
        PipelineStateObject {
            vs_desc: vk::PipelineShaderStageCreateInfo{..Default::default()},
            ps_desc: vk::PipelineShaderStageCreateInfo{..Default::default()},
            pipeline: None,
            attachment_desc: vec![],
            render_pass: None,
            input_layout_desc: vec![],
            viewport: vk::Rect2D{..Default::default()},
            scissor: vk::Rect2D{..Default::default()},
            vs_mod: None,
            ps_mod: None
        }
    }
}

impl PipelineStateObject {
    pub fn builer() -> Box<PipelineStateObject>{
        let vs_desc = vk::PipelineShaderStageCreateInfo{..Default::default()};
        let ps_desc = vk::PipelineShaderStageCreateInfo{..Default::default()};
        

        Box::new(PipelineStateObject{
            ..Default::default()
        })
    }
}


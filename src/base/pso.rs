use ash::{vk, util::*};
use std::boxed::Box;
#[derive(Clone, Debug)]
pub struct PipelineStateObjectDescriptor {
    pub vs_desc: vk::PipelineShaderStageCreateInfo,
    pub ps_desc: vk::PipelineShaderStageCreateInfo,
    pub attachment_desc: Vec<vk::AttachmentDescription>,
    pub input_layout_desc: Vec<vk::VertexInputAttributeDescription>,
    pub viewport: vk::Rect2D,
    pub scissor: vk::Rect2D,
}

impl ::std::default::Default for PipelineStateObjectDescriptor {
    fn default() -> Self {
        PipelineStateObjectDescriptor {
            vs_desc: vk::PipelineShaderStageCreateInfo{..Default::default()},
            ps_desc: vk::PipelineShaderStageCreateInfo{..Default::default()},
            attachment_desc: vec![],
            input_layout_desc: vec![],
            viewport: vk::Rect2D{..Default::default()},
            scissor: vk::Rect2D{..Default::default()},
        }
    }
}

pub struct PipelineStateObject {
    pub vs_mod: vk::ShaderModule,
    pub ps_mod: vk::ShaderModule,
    pub render_pass: vk::RenderPass,
    pub sub_passes: Vec<vk::SubpassDescription>,
}

use super::VulkanDevice;
use ash::vk;

pub struct VulkanShader
{
	pub module: vk::ShaderModule,
	pub code: Vec<u32>,
}

impl VulkanDevice
{
	pub fn create_shader(&self, data: &[u32]) -> VulkanShader
	{
		let module = unsafe {
			self.raw
				.create_shader_module(&vk::ShaderModuleCreateInfo::builder().code(data), None)
				.expect("Failed to create shader!")
		};

		VulkanShader {
			module,
			code: data.to_vec(),
		}
	}

	pub fn destroy_shader(&self, shader: VulkanShader)
	{
		unsafe {
			self.raw.destroy_shader_module(shader.module, None);
		}
	}
}

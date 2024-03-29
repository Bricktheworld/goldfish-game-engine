use super::device::{VulkanDestructor, VulkanDevice};
use crate::renderer::{TextureFormat, TextureUsage};
use ash::vk;
use gpu_allocator::vulkan as vma;
use gpu_allocator::MemoryLocation;
use std::hash::{Hash, Hasher};

pub struct VulkanTexture {
	pub width: u32,
	pub height: u32,

	pub image: vk::Image,
	pub sampler: vk::Sampler,
	pub image_view: vk::ImageView,
	pub subresource_range: vk::ImageSubresourceRange,

	pub allocation: vma::Allocation,
	pub format: TextureFormat,
	pub usage: TextureUsage,
}

impl Hash for VulkanTexture {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.image.hash(state);
		self.sampler.hash(state);
		self.image_view.hash(state);
	}
}

impl PartialEq for VulkanTexture {
	fn eq(&self, other: &Self) -> bool {
		self.image == other.image && self.sampler == other.sampler && self.image_view == other.image_view
	}
}

impl Eq for VulkanTexture {}

impl VulkanDevice {
	pub fn create_texture(&self, width: u32, height: u32, format: TextureFormat, usage: TextureUsage) -> VulkanTexture {
		let mut usage_flags = vk::ImageUsageFlags::default();

		if usage.contains(TextureUsage::ATTACHMENT) {
			if format == TextureFormat::Depth {
				usage_flags |= vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT;
			} else {
				usage_flags |= vk::ImageUsageFlags::COLOR_ATTACHMENT;
			}
		}

		if usage.contains(TextureUsage::SAMPLED) {
			usage_flags |= vk::ImageUsageFlags::SAMPLED;
		}

		if usage.contains(TextureUsage::TRANSFER_SRC) {
			usage_flags |= vk::ImageUsageFlags::TRANSFER_SRC;
		}

		if usage.contains(TextureUsage::TRANSFER_DST) {
			usage_flags |= vk::ImageUsageFlags::TRANSFER_DST;
		}

		if usage.contains(TextureUsage::STORAGE) {
			usage_flags |= vk::ImageUsageFlags::STORAGE;
		}

		let mut guard = self.vma.lock().unwrap();
		let vma = guard.as_mut().unwrap();

		let vk_format = format.to_vk(self);

		let image = unsafe {
			self.raw
				.create_image(
					&vk::ImageCreateInfo::builder()
						.flags(if format.is_cubemap() {
							vk::ImageCreateFlags::CUBE_COMPATIBLE
						} else {
							vk::ImageCreateFlags::default()
						})
						.image_type(vk::ImageType::TYPE_2D)
						.format(vk_format)
						.extent(vk::Extent3D { width, height, depth: 1 })
						.mip_levels(1)
						.array_layers(if format.is_cubemap() { 6 } else { 1 })
						.samples(vk::SampleCountFlags::TYPE_1)
						.tiling(vk::ImageTiling::OPTIMAL)
						.usage(usage_flags)
						.sharing_mode(vk::SharingMode::EXCLUSIVE)
						.initial_layout(vk::ImageLayout::UNDEFINED),
					None,
				)
				.expect("Failed to create image!")
		};

		let requirements = unsafe { self.raw.get_image_memory_requirements(image) };

		let allocation = vma
			.allocate(&vma::AllocationCreateDesc {
				name: "Texture",
				requirements,
				location: MemoryLocation::GpuOnly,
				linear: false,
			})
			.expect("Failed to allocate memory!");

		unsafe {
			self.raw.bind_image_memory(image, allocation.memory(), allocation.offset()).expect("Failed to bind image memory!");
		}

		let sampler = unsafe {
			self.raw
				.create_sampler(
					&vk::SamplerCreateInfo::builder()
						.mag_filter(vk::Filter::LINEAR)
						.min_filter(vk::Filter::LINEAR)
						.mipmap_mode(vk::SamplerMipmapMode::LINEAR)
						.address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
						.address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
						.address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE)
						.mip_lod_bias(0.0)
						.max_anisotropy(1.0)
						.min_lod(0.0)
						.max_lod(0.0)
						.border_color(vk::BorderColor::FLOAT_OPAQUE_WHITE),
					None,
				)
				.expect("Failed to create sampler!")
		};

		let subresource_range = vk::ImageSubresourceRange::builder()
			.aspect_mask(match format {
				TextureFormat::Depth => vk::ImageAspectFlags::DEPTH, // TODO(Brandon): We need to figure out a good way to handle multiple image views + samplers for depth + stencil attachments
				_ => vk::ImageAspectFlags::COLOR,
			})
			.base_mip_level(0)
			.level_count(1)
			.base_array_layer(0)
			.layer_count(if format.is_cubemap() { 6 } else { 1 })
			.build();

		let image_view = unsafe {
			self.raw
				.create_image_view(
					&vk::ImageViewCreateInfo::builder()
						.image(image)
						.view_type(if format.is_cubemap() { vk::ImageViewType::CUBE } else { vk::ImageViewType::TYPE_2D })
						.format(vk_format)
						.subresource_range(subresource_range),
					None,
				)
				.expect("Failed to create image view!")
		};

		VulkanTexture {
			width,
			height,

			image,
			sampler,
			image_view,
			subresource_range,

			allocation,
			format,
			usage,
		}
	}

	pub fn destroy_texture(&mut self, texture: VulkanTexture) {
		self.queue_destruction(&mut [
			VulkanDestructor::Image(texture.image),
			VulkanDestructor::ImageView(texture.image_view),
			VulkanDestructor::Sampler(texture.sampler),
			VulkanDestructor::Allocation(texture.allocation),
		])
	}
}

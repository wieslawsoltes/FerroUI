use super::VulkanCommandBuffer;
use crate::{IVulkanPlatformGraphicsContext, VulkanHelpers};
use ash::vk;

pub(crate) struct VulkanMemoryHelper;

impl VulkanMemoryHelper {
    /// The index of the first memory type of the physical device that is
    /// allowed by `memory_type_bits` and has all of `flags`; -1 when there
    /// is none.
    pub fn find_suitable_memory_type_index(
        context: &dyn IVulkanPlatformGraphicsContext,
        memory_type_bits: u32,
        flags: vk::MemoryPropertyFlags,
    ) -> i32 {
        let properties =
            context.instance_api().get_physical_device_memory_properties(context.physical_device_handle());
        Self::find_suitable_memory_type_index_in(&properties, memory_type_bits, flags)
    }

    /// [`find_suitable_memory_type_index`](Self::find_suitable_memory_type_index)
    /// over the memory properties of a physical device.
    pub fn find_suitable_memory_type_index_in(
        properties: &vk::PhysicalDeviceMemoryProperties,
        memory_type_bits: u32,
        flags: vk::MemoryPropertyFlags,
    ) -> i32 {
        let count = (properties.memory_type_count as usize).min(properties.memory_types.len());
        for (i, memory_type) in properties.memory_types[..count].iter().enumerate() {
            if (memory_type_bits & (1 << i)) != 0 && memory_type.property_flags.contains(flags) {
                return i as i32;
            }
        }

        -1
    }

    /// Records the change of the layout of the colour aspect of all
    /// `mip_levels` of an image. The stages and access masks follow from
    /// the two layouts ([`BarrierMasks::of_layouts`]); for a pair of layouts
    /// the table does not have, the barrier covers every stage with the
    /// given access masks ([`BarrierMasks::broad`]).
    #[allow(clippy::too_many_arguments)]
    pub fn transition_layout(
        context: &dyn IVulkanPlatformGraphicsContext,
        command_buffer: &VulkanCommandBuffer,
        image: vk::Image,
        source_layout: vk::ImageLayout,
        source_access_mask: vk::AccessFlags,
        destination_layout: vk::ImageLayout,
        destination_access_mask: vk::AccessFlags,
        mip_levels: u32,
    ) {
        let masks = BarrierMasks::of_layouts(source_layout, destination_layout)
            .unwrap_or_else(|| BarrierMasks::broad(source_access_mask, destination_access_mask));
        Self::record_barrier(context, command_buffer, image, source_layout, destination_layout, masks, mip_levels);
    }

    /// Records a layout transition of the colour aspect of all `mip_levels`
    /// of an image with the given stages and access masks.
    pub fn record_barrier(
        context: &dyn IVulkanPlatformGraphicsContext,
        command_buffer: &VulkanCommandBuffer,
        image: vk::Image,
        source_layout: vk::ImageLayout,
        destination_layout: vk::ImageLayout,
        masks: BarrierMasks,
        mip_levels: u32,
    ) {
        let subresource_range = vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            level_count: mip_levels,
            layer_count: 1,
            ..Default::default()
        };

        let barrier = vk::ImageMemoryBarrier::default()
            .src_access_mask(masks.src_access)
            .dst_access_mask(masks.dst_access)
            .old_layout(source_layout)
            .new_layout(destination_layout)
            .src_queue_family_index(VulkanHelpers::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(VulkanHelpers::QUEUE_FAMILY_IGNORED)
            .image(image)
            .subresource_range(subresource_range);

        context.device_api().cmd_pipeline_barrier(
            command_buffer.handle(),
            masks.src_stage,
            masks.dst_stage,
            vk::DependencyFlags::empty(),
            &[barrier],
        );
    }
}

/// The stages and access masks of an image barrier: the work before it that
/// has to finish (`src_stage`) and whose writes have to be made available
/// (`src_access`), and the work after it that waits (`dst_stage`) and sees
/// them (`dst_access`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BarrierMasks {
    pub src_stage: vk::PipelineStageFlags,
    pub src_access: vk::AccessFlags,
    pub dst_stage: vk::PipelineStageFlags,
    pub dst_access: vk::AccessFlags,
}

impl BarrierMasks {
    /// The masks of the transitions of the present path and of a new image,
    /// from the layouts alone:
    ///
    /// - undefined or present to transfer destination (the acquired image
    ///   of the swap chain): after the transfer stage, which is where the
    ///   presentation waits for the acquire semaphore, so the transition is
    ///   ordered after the acquisition; before the transfer that writes it.
    /// - undefined to colour attachment (a new image): from the top of the
    ///   pipe, nothing to wait for; before the colour attachment output.
    /// - colour attachment to transfer source: after the colour attachment
    ///   writes; before the transfer that reads it.
    /// - transfer source back to colour attachment: after the transfer that
    ///   read it (no write to make available); before the next frame's
    ///   colour attachment reads and writes.
    /// - transfer destination to present: after the transfer that wrote it;
    ///   nothing waits in the queue (the presentation waits for the render
    ///   finished semaphore, which makes the writes visible to it).
    ///
    /// Any other pair is not known here: `None`.
    pub fn of_layouts(source: vk::ImageLayout, destination: vk::ImageLayout) -> Option<Self> {
        use vk::AccessFlags as A;
        use vk::ImageLayout as L;
        use vk::PipelineStageFlags as S;
        let masks =
            |src_stage, src_access, dst_stage, dst_access| Self { src_stage, src_access, dst_stage, dst_access };
        let color = A::COLOR_ATTACHMENT_READ | A::COLOR_ATTACHMENT_WRITE;
        Some(match (source, destination) {
            (L::UNDEFINED | L::PRESENT_SRC_KHR, L::TRANSFER_DST_OPTIMAL) => {
                masks(S::TRANSFER, A::NONE, S::TRANSFER, A::TRANSFER_WRITE)
            }
            (L::UNDEFINED, L::COLOR_ATTACHMENT_OPTIMAL) => {
                masks(S::TOP_OF_PIPE, A::NONE, S::COLOR_ATTACHMENT_OUTPUT, color)
            }
            (L::COLOR_ATTACHMENT_OPTIMAL, L::TRANSFER_SRC_OPTIMAL) => {
                masks(S::COLOR_ATTACHMENT_OUTPUT, A::COLOR_ATTACHMENT_WRITE, S::TRANSFER, A::TRANSFER_READ)
            }
            (L::TRANSFER_SRC_OPTIMAL, L::COLOR_ATTACHMENT_OPTIMAL) => {
                masks(S::TRANSFER, A::NONE, S::COLOR_ATTACHMENT_OUTPUT, color)
            }
            (L::TRANSFER_DST_OPTIMAL, L::PRESENT_SRC_KHR) => {
                masks(S::TRANSFER, A::TRANSFER_WRITE, S::BOTTOM_OF_PIPE, A::NONE)
            }
            _ => return None,
        })
    }

    /// Every stage before and after, with the given access masks: for a
    /// transition whose earlier or later user is not known here, such as an
    /// image another user of its memory brought into its layout.
    pub fn broad(src_access: vk::AccessFlags, dst_access: vk::AccessFlags) -> Self {
        Self {
            src_stage: vk::PipelineStageFlags::ALL_COMMANDS,
            src_access,
            dst_stage: vk::PipelineStageFlags::ALL_COMMANDS,
            dst_access,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn properties(types: &[vk::MemoryPropertyFlags]) -> vk::PhysicalDeviceMemoryProperties {
        let mut properties = vk::PhysicalDeviceMemoryProperties::default();
        properties.memory_type_count = types.len() as u32;
        for (i, flags) in types.iter().enumerate() {
            properties.memory_types[i].property_flags = *flags;
        }
        properties
    }

    #[test]
    fn the_masks_of_a_transition_follow_from_its_layouts() {
        use vk::AccessFlags as A;
        use vk::ImageLayout as L;
        use vk::PipelineStageFlags as S;
        let color = A::COLOR_ATTACHMENT_READ | A::COLOR_ATTACHMENT_WRITE;
        let table = [
            (L::UNDEFINED, L::TRANSFER_DST_OPTIMAL, S::TRANSFER, A::NONE, S::TRANSFER, A::TRANSFER_WRITE),
            (L::PRESENT_SRC_KHR, L::TRANSFER_DST_OPTIMAL, S::TRANSFER, A::NONE, S::TRANSFER, A::TRANSFER_WRITE),
            (L::UNDEFINED, L::COLOR_ATTACHMENT_OPTIMAL, S::TOP_OF_PIPE, A::NONE, S::COLOR_ATTACHMENT_OUTPUT, color),
            (
                L::COLOR_ATTACHMENT_OPTIMAL,
                L::TRANSFER_SRC_OPTIMAL,
                S::COLOR_ATTACHMENT_OUTPUT,
                A::COLOR_ATTACHMENT_WRITE,
                S::TRANSFER,
                A::TRANSFER_READ,
            ),
            (
                L::TRANSFER_SRC_OPTIMAL,
                L::COLOR_ATTACHMENT_OPTIMAL,
                S::TRANSFER,
                A::NONE,
                S::COLOR_ATTACHMENT_OUTPUT,
                color,
            ),
            (L::TRANSFER_DST_OPTIMAL, L::PRESENT_SRC_KHR, S::TRANSFER, A::TRANSFER_WRITE, S::BOTTOM_OF_PIPE, A::NONE),
        ];
        for (source, destination, src_stage, src_access, dst_stage, dst_access) in table {
            assert_eq!(
                BarrierMasks::of_layouts(source, destination),
                Some(BarrierMasks { src_stage, src_access, dst_stage, dst_access }),
                "{source:?} to {destination:?}"
            );
        }
    }

    #[test]
    fn no_transition_of_the_present_path_waits_for_all_commands() {
        use vk::ImageLayout as L;
        let present_path = [
            (L::UNDEFINED, L::TRANSFER_DST_OPTIMAL),
            (L::COLOR_ATTACHMENT_OPTIMAL, L::TRANSFER_SRC_OPTIMAL),
            (L::TRANSFER_SRC_OPTIMAL, L::COLOR_ATTACHMENT_OPTIMAL),
            (L::TRANSFER_DST_OPTIMAL, L::PRESENT_SRC_KHR),
        ];
        for (source, destination) in present_path {
            let masks = BarrierMasks::of_layouts(source, destination).expect("a transition of the present path");
            assert!(!masks.src_stage.contains(vk::PipelineStageFlags::ALL_COMMANDS));
            assert!(!masks.dst_stage.contains(vk::PipelineStageFlags::ALL_COMMANDS));
        }
    }

    #[test]
    fn an_unknown_transition_is_broad_with_the_given_access() {
        use vk::ImageLayout as L;
        assert_eq!(BarrierMasks::of_layouts(L::GENERAL, L::TRANSFER_SRC_OPTIMAL), None);
        assert_eq!(BarrierMasks::of_layouts(L::SHADER_READ_ONLY_OPTIMAL, L::COLOR_ATTACHMENT_OPTIMAL), None);
        let broad = BarrierMasks::broad(vk::AccessFlags::SHADER_WRITE, vk::AccessFlags::TRANSFER_READ);
        assert_eq!(broad.src_stage, vk::PipelineStageFlags::ALL_COMMANDS);
        assert_eq!(broad.dst_stage, vk::PipelineStageFlags::ALL_COMMANDS);
        assert_eq!(broad.src_access, vk::AccessFlags::SHADER_WRITE);
        assert_eq!(broad.dst_access, vk::AccessFlags::TRANSFER_READ);
    }

    #[test]
    fn the_first_allowed_type_with_all_flags_is_taken() {
        let local = vk::MemoryPropertyFlags::DEVICE_LOCAL;
        let host = vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT;
        let properties = properties(&[host, local, local | host]);

        assert_eq!(VulkanMemoryHelper::find_suitable_memory_type_index_in(&properties, 0b111, local), 1);
        // The type is not among the allowed ones: the next one with the flags.
        assert_eq!(VulkanMemoryHelper::find_suitable_memory_type_index_in(&properties, 0b101, local), 2);
        // All of the flags have to be there.
        assert_eq!(VulkanMemoryHelper::find_suitable_memory_type_index_in(&properties, 0b011, local | host), -1);
        assert_eq!(VulkanMemoryHelper::find_suitable_memory_type_index_in(&properties, 0b111, local | host), 2);
    }

    #[test]
    fn no_type_is_minus_one() {
        let properties = properties(&[]);
        assert_eq!(
            VulkanMemoryHelper::find_suitable_memory_type_index_in(
                &properties,
                u32::MAX,
                vk::MemoryPropertyFlags::DEVICE_LOCAL
            ),
            -1
        );
    }

    #[test]
    fn types_beyond_the_count_are_not_read() {
        let mut properties = properties(&[vk::MemoryPropertyFlags::HOST_VISIBLE]);
        properties.memory_types[1].property_flags = vk::MemoryPropertyFlags::DEVICE_LOCAL;
        assert_eq!(
            VulkanMemoryHelper::find_suitable_memory_type_index_in(
                &properties,
                u32::MAX,
                vk::MemoryPropertyFlags::DEVICE_LOCAL
            ),
            -1
        );
    }
}

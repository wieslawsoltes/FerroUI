// rust-skia 0.153 exposes Graphite surfaces, but does not bind MakeVulkan
// for textures or the target layout fields of InsertRecordingInfo.
#include "include/gpu/graphite/Context.h"
#include "include/gpu/graphite/vk/VulkanGraphiteTypes.h"
#include "include/gpu/vk/VulkanMutableTextureState.h"

using namespace skgpu::graphite;

extern "C" void ferro_graphite_wrap(
        BackendTexture* texture, int width, int height, uint64_t image, uint64_t memory,
        uint64_t bytes, uint32_t format, uint32_t usage, uint32_t tiling, uint32_t layout,
        uint32_t queue, uint32_t levels, uint32_t samples, bool protectedImage) {
    skgpu::VulkanAlloc alloc;
    alloc.fMemory = (VkDeviceMemory)memory;
    alloc.fSize = bytes;
    VulkanTextureInfo info(static_cast<VkSampleCountFlagBits>(samples),
                           levels > 1 ? skgpu::Mipmapped::kYes : skgpu::Mipmapped::kNo,
                           protectedImage ? VK_IMAGE_CREATE_PROTECTED_BIT : 0,
                           static_cast<VkFormat>(format), static_cast<VkImageTiling>(tiling),
                           usage, VK_SHARING_MODE_EXCLUSIVE, VK_IMAGE_ASPECT_COLOR_BIT, {});
    *texture = BackendTextures::MakeVulkan({width, height}, info, static_cast<VkImageLayout>(layout),
                                          queue, (VkImage)image, alloc);
}

extern "C" bool ferro_graphite_submit(Context* context, Recording* recording, SkSurface* surface,
                                      uint32_t layout, uint32_t queue) {
    auto state = skgpu::MutableTextureStates::MakeVulkan(static_cast<VkImageLayout>(layout), queue);
    InsertRecordingInfo info;
    info.fRecording = recording;
    info.fTargetSurface = surface;
    info.fTargetTextureState = &state;
    return context->insertRecording(info) && context->submit();
}

//! Port of `VulkanDemo/VulkanCommandBufferPool.cs` (a copy of the command buffer pool of the
//! framework's Vulkan backend, in its namespace upstream): a command pool, the command buffers
//! it hands out, and the ones submitted, which are freed once their fences are signalled.
//!
//! The pool and its buffers are used on the thread of the demo; upstream guards the list of the
//! used buffers with a lock all the same, which is a `RefCell` here.

use super::vulkan_extensions::{ThrowOnError, VulkanDemoResult};
use ash::vk;
use std::cell::{Cell, RefCell};
use std::ffi::c_void;
use std::rc::Rc;

/// What the pool and the buffers it hands out share.
struct PoolInner {
    device: ash::Device,
    queue: vk::Queue,
    command_pool: vk::CommandPool,
    used_command_buffers: RefCell<Vec<VulkanCommandBuffer>>,
}

/// A clone is the same pool.
#[derive(Clone)]
pub struct VulkanCommandBufferPool {
    inner: Rc<PoolInner>,
}

impl VulkanCommandBufferPool {
    pub fn new(device: &ash::Device, queue: vk::Queue, queue_family_index: u32) -> VulkanDemoResult<Self> {
        let command_pool_create_info = vk::CommandPoolCreateInfo::default()
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .queue_family_index(queue_family_index);

        // SAFETY: the device is alive and the structure is valid.
        let command_pool = unsafe { device.create_command_pool(&command_pool_create_info, None) }.throw_on_error()?;

        Ok(Self {
            inner: Rc::new(PoolInner {
                device: device.clone(),
                queue,
                command_pool,
                used_command_buffers: RefCell::new(Vec::new()),
            }),
        })
    }

    pub fn dispose(&self) {
        self.free_used_command_buffers();
        // SAFETY: every buffer of the pool that was submitted is freed above, after its fence.
        unsafe { self.inner.device.destroy_command_pool(self.inner.command_pool, None) };
    }

    fn allocate_command_buffer(inner: &PoolInner) -> vk::CommandBuffer {
        let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(inner.command_pool)
            .command_buffer_count(1)
            .level(vk::CommandBufferLevel::PRIMARY);

        // Upstream ignores the result of the call and takes the handle it wrote, which is null
        // on a failure; so does this.
        // SAFETY: the pool is alive and the structure is valid.
        unsafe { inner.device.allocate_command_buffers(&command_buffer_allocate_info) }
            .ok()
            .and_then(|buffers| buffers.first().copied())
            .unwrap_or_default()
    }

    pub fn create_command_buffer(&self) -> VulkanCommandBuffer {
        VulkanCommandBuffer::new(self.inner.clone())
    }

    pub fn free_used_command_buffers(&self) {
        let used = std::mem::take(&mut *self.inner.used_command_buffers.borrow_mut());
        for used_command_buffer in used {
            used_command_buffer.dispose();
        }
    }
}

/// `KeyedMutexSubmitInfo`: the keys of the keyed mutex of an image that is shared with
/// Direct3D 11, acquired before and released after a submission.
#[derive(Clone, Copy, Debug, Default)]
pub struct KeyedMutexSubmitInfo {
    pub acquire_key: Option<u64>,
    pub release_key: Option<u64>,
    pub device_memory: vk::DeviceMemory,
}

/// A command buffer of the pool, with the fence of its submission.
pub struct VulkanCommandBuffer {
    command_buffer_pool: Rc<PoolInner>,
    fence: vk::Fence,
    has_ended: Cell<bool>,
    has_started: Cell<bool>,
    internal_handle: vk::CommandBuffer,
}

impl VulkanCommandBuffer {
    fn new(command_buffer_pool: Rc<PoolInner>) -> Self {
        let internal_handle = VulkanCommandBufferPool::allocate_command_buffer(&command_buffer_pool);

        let fence_create_info = vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED);

        // Upstream ignores the result here too.
        // SAFETY: the device is alive and the structure is valid.
        let fence =
            unsafe { command_buffer_pool.device.create_fence(&fence_create_info, None) }.unwrap_or_default();

        Self { command_buffer_pool, fence, has_ended: Cell::new(false), has_started: Cell::new(false), internal_handle }
    }

    /// The handle of the command buffer (`Handle` and `InternalHandle` upstream).
    pub fn internal_handle(&self) -> vk::CommandBuffer {
        self.internal_handle
    }

    fn dispose(self) {
        let pool = &self.command_buffer_pool;
        // SAFETY: the fence and the buffer belong to the device of the pool; the buffer is freed
        // once its submission has completed.
        unsafe {
            let _ = pool.device.wait_for_fences(&[self.fence], true, u64::MAX);
            pool.device.free_command_buffers(pool.command_pool, &[self.internal_handle]);
            pool.device.destroy_fence(self.fence, None);
        }
    }

    pub fn begin_recording(&self) {
        if !self.has_started.get() {
            self.has_started.set(true);

            let begin_info =
                vk::CommandBufferBeginInfo::default().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

            // SAFETY: the buffer is not recording yet.
            let _ = unsafe { self.command_buffer_pool.device.begin_command_buffer(self.internal_handle, &begin_info) };
        }
    }

    pub fn end_recording(&self) {
        if self.has_started.get() && !self.has_ended.get() {
            self.has_ended.set(true);

            // SAFETY: the buffer is recording.
            let _ = unsafe { self.command_buffer_pool.device.end_command_buffer(self.internal_handle) };
        }
    }

    /// Submits the buffer with its own fence and nothing to wait for or signal.
    pub fn submit(self) {
        self.submit_with(&[], &[], &[], None, None, std::ptr::null());
    }

    /// Submits the buffer: it waits for `wait_semaphores` (at the stages of
    /// `wait_dst_stage_mask`), signals `signal_semaphores` and `fence` (its own fence by
    /// default), acquires and releases the keys of `keyed_mutex`, and has `p_next` chained to
    /// the submission. The buffer joins the used buffers of the pool.
    pub fn submit_with(
        self,
        wait_semaphores: &[vk::Semaphore],
        wait_dst_stage_mask: &[vk::PipelineStageFlags],
        signal_semaphores: &[vk::Semaphore],
        fence: Option<vk::Fence>,
        keyed_mutex: Option<&KeyedMutexSubmitInfo>,
        p_next: *const c_void,
    ) {
        self.end_recording();

        let fence = fence.unwrap_or(self.fence);

        let acquire_key = keyed_mutex.and_then(|m| m.acquire_key).unwrap_or(0);
        let release_key = keyed_mutex.and_then(|m| m.release_key).unwrap_or(0);
        let dev_mem = keyed_mutex.map(|m| m.device_memory).unwrap_or_default();
        let timeout = u32::MAX;
        let mut mutex = vk::Win32KeyedMutexAcquireReleaseInfoKHR::default();
        if let Some(keyed_mutex) = keyed_mutex {
            mutex.acquire_count = u32::from(keyed_mutex.acquire_key.is_some());
            mutex.release_count = u32::from(keyed_mutex.release_key.is_some());
            mutex.p_acquire_keys = &acquire_key;
            mutex.p_release_keys = &release_key;
            mutex.p_acquire_syncs = &dev_mem;
            mutex.p_release_syncs = &dev_mem;
            mutex.p_acquire_timeouts = &timeout;
            mutex.p_next = p_next;
        }

        let command_buffer = self.internal_handle;
        let submit_info = vk::SubmitInfo {
            p_next: if keyed_mutex.is_some() { (&mutex as *const vk::Win32KeyedMutexAcquireReleaseInfoKHR).cast() } else { p_next },
            wait_semaphore_count: wait_semaphores.len() as u32,
            p_wait_semaphores: if wait_semaphores.is_empty() { std::ptr::null() } else { wait_semaphores.as_ptr() },
            p_wait_dst_stage_mask: if wait_dst_stage_mask.is_empty() {
                std::ptr::null()
            } else {
                wait_dst_stage_mask.as_ptr()
            },
            command_buffer_count: 1,
            p_command_buffers: &command_buffer,
            signal_semaphore_count: signal_semaphores.len() as u32,
            p_signal_semaphores: if signal_semaphores.is_empty() { std::ptr::null() } else { signal_semaphores.as_ptr() },
            ..Default::default()
        };

        let device = &self.command_buffer_pool.device;
        // SAFETY: every pointer of the structures points to a value of this frame that outlives
        // the call, and the chain `p_next` the caller passes is valid for the call.
        unsafe {
            let _ = device.reset_fences(&[fence]);
            let _ = device.queue_submit(self.command_buffer_pool.queue, &[submit_info], fence);
        }

        let pool = self.command_buffer_pool.clone();
        pool.used_command_buffers.borrow_mut().push(self);
    }
}

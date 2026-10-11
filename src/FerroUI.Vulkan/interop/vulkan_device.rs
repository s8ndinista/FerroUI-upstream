use crate::unmanaged_interop::VulkanInstanceApi;
use crate::{IVulkanDevice, IVulkanInstance};
use ash::vk;
use ferroui_base::platform::IOptionalFeatureProvider;
use ferroui_base::reactive::{Disposable, IDisposable};
use std::any::{Any, TypeId};
use std::rc::Rc;
use std::sync::{Arc, Condvar, Mutex, Weak};
use std::thread::{self, ThreadId};

/// A device the platform graphics created (`vulkan_device_create.rs` has
/// the creation).
pub(crate) struct VulkanDevice {
    instance_api: VulkanInstanceApi,
    handle: Mutex<vk::Device>,
    physical_device_handle: vk::PhysicalDevice,
    main_queue: vk::Queue,
    graphics_queue_index: u32,
    lock: Mutex<LockState>,
    unlocked: Condvar,
    instance: Arc<dyn IVulkanInstance>,
    enabled_extensions: Vec<String>,
    this: Weak<VulkanDevice>,
}

/// Which thread has the device and how many times it took it.
#[derive(Default)]
struct LockState {
    locked_by_thread: Option<ThreadId>,
    lock_count: u32,
}

impl VulkanDevice {
    pub(super) fn new(
        instance_api: VulkanInstanceApi,
        handle: vk::Device,
        physical_device_handle: vk::PhysicalDevice,
        main_queue: vk::Queue,
        graphics_queue_index: u32,
        enabled_extensions: Vec<String>,
    ) -> Arc<Self> {
        let instance = instance_api.instance().clone();
        Arc::new_cyclic(|this| Self {
            instance_api,
            handle: Mutex::new(handle),
            physical_device_handle,
            main_queue,
            graphics_queue_index,
            lock: Mutex::new(LockState::default()),
            unlocked: Condvar::new(),
            instance,
            enabled_extensions,
            this: this.clone(),
        })
    }

    fn lock_state(&self) -> std::sync::MutexGuard<'_, LockState> {
        // The state is two plain values: a thread that panicked while it
        // held the guard left them consistent.
        self.lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// # Panics
    /// Panics when the calling thread has not locked the device.
    fn check_access<T>(&self, f: T) -> T {
        if self.lock_state().locked_by_thread != Some(thread::current().id()) {
            panic!("This class is only usable when locked");
        }
        f
    }

    fn unlock(&self) {
        let mut state = self.lock_state();
        state.lock_count -= 1;
        if state.lock_count == 0 {
            state.locked_by_thread = None;
            self.unlocked.notify_one();
        }
    }
}

impl IVulkanDevice for VulkanDevice {
    fn lock(&self) -> Rc<dyn IDisposable> {
        let current = thread::current().id();
        {
            let mut state = self.lock_state();
            while state.locked_by_thread.is_some_and(|thread| thread != current) {
                state = self.unlocked.wait(state).unwrap_or_else(|poisoned| poisoned.into_inner());
            }
            state.lock_count += 1;
            state.locked_by_thread = Some(current);
        }
        // The result is an `Rc`: it stays on the thread that locked.
        let this = self.this.upgrade().expect("a device that is called is alive");
        Disposable::create(move || this.unlock())
    }

    fn enabled_extensions(&self) -> Vec<String> {
        self.enabled_extensions.clone()
    }

    fn is_lost(&self) -> bool {
        false
    }

    fn handle(&self) -> isize {
        let handle = *self.handle.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        vk::Handle::as_raw(handle) as isize
    }

    fn physical_device_handle(&self) -> isize {
        vk::Handle::as_raw(self.physical_device_handle) as isize
    }

    fn main_queue_handle(&self) -> isize {
        vk::Handle::as_raw(self.check_access(self.main_queue)) as isize
    }

    fn graphics_queue_family_index(&self) -> u32 {
        self.graphics_queue_index
    }

    fn instance(&self) -> Arc<dyn IVulkanInstance> {
        self.instance.clone()
    }

    fn dispose(&self) {
        let mut handle = self.handle.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if *handle != vk::Device::null() {
            self.instance_api.destroy_device(*handle);
            *handle = vk::Device::null();
        }
    }
}

impl IOptionalFeatureProvider for VulkanDevice {
    fn try_get_feature(&self, _feature_type: TypeId) -> Option<Rc<dyn Any>> {
        None
    }
}

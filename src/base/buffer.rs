use ash;
use ash::vk;
use ash::version::{DeviceV1_0, InstanceV1_0};
use crate::base::ri;
use crate::base::utility::find_memorytype_index;
use std::rc;

static BUFFER_ALIGN: u64 = 4; // 4 bytes
pub struct DeviceBuffer {
    pub buffer: vk::Buffer,
    pub memory: vk::DeviceMemory,
    pub device: ash::Device,
    size: u64,
    offset: u64,
}

impl DeviceBuffer {
    pub fn new(device: &ash::Device, device_memory_properties: &vk::PhysicalDeviceMemoryProperties,
            buffer_ci: &vk::BufferCreateInfo,
            flags: vk::MemoryPropertyFlags)
    -> DeviceBuffer
    {
        // create buffer
        let buffer;
        unsafe {
            buffer = device.create_buffer(&buffer_ci, None)
                .unwrap();
        }
        // create memory
        let memory;
        unsafe {
            let memory_req = device.get_buffer_memory_requirements(buffer);

            let memory_allocate_ci = vk::MemoryAllocateInfo {
                allocation_size: memory_req.size,
                memory_type_index: find_memorytype_index(
                    &memory_req,
                    &device_memory_properties,
                    flags,
                ).unwrap(),
                ..Default::default()
            };
            memory = device.allocate_memory(&memory_allocate_ci, None)
                .unwrap();
        }
        // bind memory to buffer
        unsafe {
            device.unmap_memory(memory);
            device.bind_buffer_memory(buffer, memory, 0).unwrap();
        }

        DeviceBuffer {
            buffer,
            memory,
            device: device.clone(),
            size: buffer_ci.size,
            offset: 0,
        }
    }

    pub fn clear(&mut self) {
        self.offset = 0;
    }

    pub fn allocate<T>(&mut self, size: u64)
        -> BufferSlice<T>
    {
        let start = self.offset;
        let new_offset = self.offset + (size + BUFFER_ALIGN - 1) / BUFFER_ALIGN * BUFFER_ALIGN;
        assert!(new_offset <= self.size, "buffer size is over");
        self.offset = new_offset;
        let truth_size = self.offset - start;
        let slice;
        unsafe {
            let buf_ptr = self.device
                .map_memory(
                    self.memory,
                    start,
                    truth_size,
                    vk::MemoryMapFlags::empty(),
                )
                .unwrap();

            slice = ash::util::Align::new(
                buf_ptr,
                std::mem::align_of::<T>() as u64,
                truth_size
            ) as ash::util::Align<T>;
        }
        BufferSlice {
            buffer: self.buffer,
            offset: start,
            size: truth_size,
            slice
        }
    }
}

impl Drop for DeviceBuffer {
    fn drop(&mut self) {
        unsafe {
            self.device.free_memory(self.memory, None);
            self.device.destroy_buffer(self.buffer, None);
        }
        self.memory = vk::DeviceMemory::default();
        self.buffer = vk::Buffer::default();
    }
}

pub struct BufferSlice<T> {
    pub buffer: vk::Buffer,
    pub offset: u64,
    pub size: u64,
    pub slice: ash::util::Align<T>,
}

pub struct BufferManagerSystem {
    pub index_buf_size: u64,
    pub index_buffer: DeviceBuffer,
    pub vertex_buf_size: u64,
    pub vertex_buffer: DeviceBuffer,
    pub uniform_buf_size: u64,
    pub uniform_buffer: DeviceBuffer,
}

impl BufferManagerSystem {

    pub fn new(backend: &rc::Rc<ri::Backend>, vertex_buf_size: vk::DeviceSize,
               index_buf_size: vk::DeviceSize, uniform_buf_size: vk::DeviceSize)
        -> BufferManagerSystem
    {
        let device_memory_properties = unsafe {
            backend.instance.get_physical_device_memory_properties(backend.physical_device)
        };
        // buffer
        let vertex_buffer_ci = vk::BufferCreateInfo::builder()
            .size(vertex_buf_size)
            .usage(vk::BufferUsageFlags::VERTEX_BUFFER)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .build();
        let vertex_buffer = DeviceBuffer::new(
            &backend.device,
            &device_memory_properties,
            &vertex_buffer_ci,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        );
        let index_buffer_ci = vk::BufferCreateInfo::builder()
            .size(index_buf_size)
            .usage(vk::BufferUsageFlags::INDEX_BUFFER)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .build();
        let index_buffer = DeviceBuffer::new(
            &backend.device,
            &device_memory_properties,
            &index_buffer_ci,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        );
        let uniform_buffer_ci = vk::BufferCreateInfo::builder()
            .size(uniform_buf_size)
            .usage(vk::BufferUsageFlags::UNIFORM_BUFFER)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .build();
        let uniform_buffer = DeviceBuffer::new(
            &backend.device,
            &device_memory_properties,
            &uniform_buffer_ci,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        );
        BufferManagerSystem {
            index_buf_size,
            index_buffer,
            vertex_buf_size,
            vertex_buffer,
            uniform_buf_size,
            uniform_buffer
        }
    }

    pub fn allocate_vertex_buffer<T>(&mut self, size: u64)
        -> BufferSlice<T>
    {
        self.vertex_buffer.allocate::<T>(size)
    }

    pub fn allocate_index_buffer<T>(&mut self, size: u64)
        -> BufferSlice<T>
    {
        self.index_buffer.allocate::<T>(size)
    }

    pub fn allocate_uniform_buffer<T>(&mut self, size: u64)
        -> BufferSlice<T>
    {
        self.uniform_buffer.allocate::<T>(size)
    }
}


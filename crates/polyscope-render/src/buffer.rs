//! GPU buffer management.

use wgpu::util::DeviceExt;

/// Creates a vertex buffer from data.
pub fn create_vertex_buffer<T: bytemuck::Pod>(
    device: &wgpu::Device,
    data: &[T],
    label: Option<&str>,
) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label,
        contents: bytemuck::cast_slice(data),
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
    })
}

/// Creates an index buffer from data.
pub fn create_index_buffer(
    device: &wgpu::Device,
    data: &[u32],
    label: Option<&str>,
) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label,
        contents: bytemuck::cast_slice(data),
        usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
    })
}

/// Creates a uniform buffer from data.
pub fn create_uniform_buffer<T: bytemuck::Pod>(
    device: &wgpu::Device,
    data: &T,
    label: Option<&str>,
) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label,
        contents: bytemuck::bytes_of(data),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    })
}

/// Creates a storage buffer from data.
pub fn create_storage_buffer<T: bytemuck::Pod>(
    device: &wgpu::Device,
    data: &[T],
    label: Option<&str>,
) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label,
        contents: bytemuck::cast_slice(data),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
    })
}

/// Updates a buffer with new data.
pub fn update_buffer<T: bytemuck::Pod>(queue: &wgpu::Queue, buffer: &wgpu::Buffer, data: &[T]) {
    queue.write_buffer(buffer, 0, bytemuck::cast_slice(data));
}

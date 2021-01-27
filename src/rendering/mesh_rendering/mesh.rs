use lyon_tessellation::VertexBuffers;

/// The vertex types describes a single point of a mesh used to form triangles.
/// It uses a C compatible layout such that it can be directly uploaded to a GPU.
#[repr(C)]
#[derive(Copy, Clone, PartialOrd, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    /// The x coordinate of the vertex.
    pub x: f32,
    /// The y coordinate of the vertex.
    pub y: f32,
    /// The u coordinate of the vertex, which corresponds to the x axis in
    /// texture space.
    pub u: f32,
    /// The v coordinate of the vertex, which corresponds to the y axis in
    /// texture space.
    pub v: f32,
}

/// A mesh supplied to the backend that will eventually be rendered out.
pub struct Mesh {
    pub(super) buffers: VertexBuffers<Vertex, u16>,
}

impl Mesh {
    pub(super) fn new() -> Self {
        Self {
            buffers: VertexBuffers::new(),
        }
    }

    /// The vertices that make up the mesh.
    pub fn vertices(&self) -> &[Vertex] {
        &self.buffers.vertices
    }

    /// The indices describe the actual triangles that make up the mesh. Each
    /// chunk of three indices pointing into the `vertices` makes up a triangle.
    pub fn indices(&self) -> &[u16] {
        &self.buffers.indices
    }

    /// The vertices that make up the mesh. This method returns the vertices as
    /// a slice of bytes, which may be easier to consume for various graphics
    /// APIs.
    pub fn vertices_as_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(self.vertices())
    }

    /// The indices describe the actual triangles that make up the mesh. Each
    /// chunk of three indices pointing into the `vertices` makes up a triangle.
    /// This method returns the indices as a slice of bytes, which may be easier
    /// to consume for various graphics APIs.
    pub fn indices_as_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(self.indices())
    }
}

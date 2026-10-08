//! Port of `Pages/OpenGl/OpenGlContent.cs`: the scene the OpenGL pages draw.

use ferroui_base::numerics::{Matrix4x4, Vector3};
use ferroui_base::platform::AssetLoader;
use ferroui_base::utilities::Uri;
use ferroui_base::PixelSize;
use ferroui_opengl::gl_consts::*;
use ferroui_opengl::{GlInterface, GlProfileType, GlVersion};
use std::ffi::c_void;
use std::io::Read;
use std::sync::LazyLock;
use std::time::Instant;

const VERTEX_SHADER_SOURCE: &str = r"
        attribute vec3 aPos;
        attribute vec3 aNormal;
        uniform mat4 uModel;
        uniform mat4 uProjection;
        uniform mat4 uView;

        varying vec3 FragPos;
        varying vec3 VecPos;  
        varying vec3 Normal;
        uniform float uTime;
        uniform float uDisco;
        void main()
        {
            float discoScale = sin(uTime * 10.0) / 10.0;
            float distortionX = 1.0 + uDisco * cos(uTime * 20.0) / 10.0;
            
            float scale = 1.0 + uDisco * discoScale;
            
            vec3 scaledPos = aPos;
            scaledPos.x = scaledPos.x * distortionX;
            
            scaledPos *= scale;
            gl_Position = uProjection * uView * uModel * vec4(scaledPos, 1.0);
            FragPos = vec3(uModel * vec4(aPos, 1.0));
            VecPos = aPos;
            Normal = normalize(vec3(uModel * vec4(aNormal, 1.0)));
        }
";

const FRAGMENT_SHADER_SOURCE: &str = r"
        varying vec3 FragPos; 
        varying vec3 VecPos; 
        varying vec3 Normal;
        uniform float uMaxY;
        uniform float uMinY;
        uniform float uTime;
        uniform float uDisco;
        //DECLAREGLFRAG

        void main()
        {
            float y = (VecPos.y - uMinY) / (uMaxY - uMinY);
            float c = cos(atan(VecPos.x, VecPos.z) * 20.0 + uTime * 40.0 + y * 50.0);
            float s = sin(-atan(VecPos.z, VecPos.x) * 20.0 - uTime * 20.0 - y * 30.0);

            vec3 discoColor = vec3(
                0.5 + abs(0.5 - y) * cos(uTime * 10.0),
                0.25 + (smoothstep(0.3, 0.8, y) * (0.5 - c / 4.0)),
                0.25 + abs((smoothstep(0.1, 0.4, y) * (0.5 - s / 4.0))));

            vec3 objectColor = vec3((1.0 - y), 0.40 +  y / 4.0, y * 0.75 + 0.25);
            objectColor = objectColor * (1.0 - uDisco) + discoColor * uDisco;

            float ambientStrength = 0.3;
            vec3 lightColor = vec3(1.0, 1.0, 1.0);
            vec3 lightPos = vec3(uMaxY * 2.0, uMaxY * 2.0, uMaxY * 2.0);
            vec3 ambient = ambientStrength * lightColor;


            vec3 norm = normalize(Normal);
            vec3 lightDir = normalize(lightPos - FragPos);  

            float diff = max(dot(norm, lightDir), 0.0);
            vec3 diffuse = diff * lightColor;

            vec3 result = (ambient + diffuse) * objectColor;
            gl_FragColor = vec4(result, 1.0);

        }
";

/// A point of the model. The layout is the one the vertex attributes are
/// declared with: the position at offset 0 and the normal at offset 12.
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Vertex {
    position: Vector3,
    normal: Vector3,
}

// `St`, the stopwatch of the class: started when the class is first used.
static ST: LazyLock<Instant> = LazyLock::new(Instant::now);

pub(crate) struct OpenGlContent {
    vertex_shader: i32,
    fragment_shader: i32,
    shader_program: i32,
    vertex_buffer_object: i32,
    index_buffer_object: i32,
    vertex_array_object: i32,
    gl_version: GlVersion,
    points: Vec<Vertex>,
    indices: Vec<u16>,
    min_y: f32,
    max_y: f32,
    info: String,
}

impl OpenGlContent {
    fn get_shader(&self, fragment: bool, shader: &str) -> String {
        let version = if self.gl_version.type_() == GlProfileType::OpenGL {
            if cfg!(target_os = "macos") {
                150
            } else {
                120
            }
        } else {
            100
        };
        let mut data = format!("#version {version}\n");
        if self.gl_version.type_() == GlProfileType::OpenGLES {
            data += "precision mediump float;\n";
        }
        let mut shader = shader.to_string();
        if version >= 150 {
            shader = shader.replace("attribute", "in");
            if fragment {
                shader = shader
                    .replace("varying", "in")
                    .replace("//DECLAREGLFRAG", "out vec4 outFragColor;")
                    .replace("gl_FragColor", "outFragColor");
            } else {
                shader = shader.replace("varying", "out");
            }
        }

        data += &shader;

        data
    }

    fn vertex_shader_source(&self) -> String {
        self.get_shader(false, VERTEX_SHADER_SOURCE)
    }

    fn fragment_shader_source(&self) -> String {
        self.get_shader(true, FRAGMENT_SHADER_SOURCE)
    }

    /// # Panics
    /// Panics if the model cannot be opened or is cut short (an exception of
    /// the constructor in the managed original).
    pub fn new() -> Self {
        LazyLock::force(&ST);

        // Deviation (DEVIATIONS.md, ControlCatalog sample): upstream finds the manifest
        // resource whose name contains "teapot.bin"; the file is an asset of the sample here.
        let uri = Uri::absolute("ferres://ControlCatalog/Pages/teapot.bin").unwrap_or_else(|e| panic!("{e}"));
        let mut sr = AssetLoader::open(&uri, None).unwrap_or_else(|e| panic!("{e}"));

        let buf = read_block(&mut sr);
        let points: Vec<f32> =
            buf.chunks_exact(4).map(|bytes| f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])).collect();
        let buf = read_block(&mut sr);
        let indices: Vec<u16> = buf.chunks_exact(2).map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]])).collect();
        let mut vertices = vec![Vertex::default(); points.len() / 3];
        for (primitive, vertex) in vertices.iter_mut().enumerate() {
            let srci = primitive * 3;
            vertex.position = Vector3::new(points[srci], points[srci + 1], points[srci + 2]);
        }

        let mut i = 0;
        while i < indices.len() {
            let a = vertices[indices[i] as usize].position;
            let b = vertices[indices[i + 1] as usize].position;
            let c = vertices[indices[i + 2] as usize].position;
            let normal = Vector3::normalize(Vector3::cross(c - b, a - b));

            vertices[indices[i] as usize].normal += normal;
            vertices[indices[i + 1] as usize].normal += normal;
            vertices[indices[i + 2] as usize].normal += normal;
            i += 3;
        }

        let mut max_y = 0.0f32;
        let mut min_y = 0.0f32;
        for vertex in vertices.iter_mut() {
            vertex.normal = Vector3::normalize(vertex.normal);
            max_y = f32::max(max_y, vertex.position.y);
            min_y = f32::min(min_y, vertex.position.y);
        }

        Self {
            vertex_shader: 0,
            fragment_shader: 0,
            shader_program: 0,
            vertex_buffer_object: 0,
            index_buffer_object: 0,
            vertex_array_object: 0,
            gl_version: GlVersion::new(GlProfileType::OpenGL, 0, 0),
            points: vertices,
            indices,
            min_y,
            max_y,
            info: String::new(),
        }
    }

    fn check_error(gl: &GlInterface) {
        loop {
            let err = gl.get_error();
            if err == GL_NO_ERROR {
                break;
            }
            println!("{err}");
        }
    }

    pub fn info(&self) -> &str {
        &self.info
    }

    pub fn init(&mut self, gl: &GlInterface, version: GlVersion) {
        self.gl_version = version;
        Self::check_error(gl);

        self.info = format!(
            "Renderer: {} Version: {}",
            gl.get_string(GL_RENDERER).unwrap_or_default(),
            gl.get_string(GL_VERSION).unwrap_or_default()
        );

        // Load the source of the vertex shader and compile it.
        self.vertex_shader = gl.create_shader(GL_VERTEX_SHADER);
        println!(
            "{}",
            gl.compile_shader_and_get_error(self.vertex_shader, &self.vertex_shader_source()).unwrap_or_default()
        );

        // Load the source of the fragment shader and compile it.
        self.fragment_shader = gl.create_shader(GL_FRAGMENT_SHADER);
        println!(
            "{}",
            gl.compile_shader_and_get_error(self.fragment_shader, &self.fragment_shader_source()).unwrap_or_default()
        );

        // Create the shader program, attach the vertex and fragment shaders and link the program.
        self.shader_program = gl.create_program();
        gl.attach_shader(self.shader_program, self.vertex_shader);
        gl.attach_shader(self.shader_program, self.fragment_shader);
        const POSITION_LOCATION: i32 = 0;
        const NORMAL_LOCATION: i32 = 1;
        gl.bind_attrib_location_string(self.shader_program, POSITION_LOCATION, "aPos");
        gl.bind_attrib_location_string(self.shader_program, NORMAL_LOCATION, "aNormal");
        println!("{}", gl.link_program_and_get_error(self.shader_program).unwrap_or_default());
        Self::check_error(gl);

        // Create the vertex buffer object (VBO) for the vertex data.
        self.vertex_buffer_object = gl.gen_buffer();
        // Bind the VBO and copy the vertex data into it.
        gl.bind_buffer(GL_ARRAY_BUFFER, self.vertex_buffer_object);
        Self::check_error(gl);
        let vertex_size = std::mem::size_of::<Vertex>();
        // SAFETY: the pointer is valid for the size passed, the whole of the vertices,
        // which OpenGL copies during the call.
        unsafe {
            gl.buffer_data(
                GL_ARRAY_BUFFER,
                (self.points.len() * vertex_size) as isize,
                self.points.as_ptr() as *const c_void,
                GL_STATIC_DRAW,
            );
        }

        self.index_buffer_object = gl.gen_buffer();
        gl.bind_buffer(GL_ELEMENT_ARRAY_BUFFER, self.index_buffer_object);
        Self::check_error(gl);
        // SAFETY: the pointer is valid for the size passed, the whole of the indices,
        // which OpenGL copies during the call.
        unsafe {
            gl.buffer_data(
                GL_ELEMENT_ARRAY_BUFFER,
                (self.indices.len() * std::mem::size_of::<u16>()) as isize,
                self.indices.as_ptr() as *const c_void,
                GL_STATIC_DRAW,
            );
        }
        Self::check_error(gl);
        self.vertex_array_object = gl.gen_vertex_array();
        gl.bind_vertex_array(self.vertex_array_object);
        Self::check_error(gl);
        // SAFETY: a buffer is bound to `GL_ARRAY_BUFFER`, so the pointers are offsets
        // into it: the position and the normal of `Vertex`.
        unsafe {
            gl.vertex_attrib_pointer(POSITION_LOCATION, 3, GL_FLOAT, 0, vertex_size as i32, std::ptr::null());
            gl.vertex_attrib_pointer(NORMAL_LOCATION, 3, GL_FLOAT, 0, vertex_size as i32, 12 as *const c_void);
        }
        gl.enable_vertex_attrib_array(POSITION_LOCATION);
        gl.enable_vertex_attrib_array(NORMAL_LOCATION);
        Self::check_error(gl);
    }

    pub fn deinit(&self, gl: &GlInterface) {
        // Unbind everything
        gl.bind_buffer(GL_ARRAY_BUFFER, 0);
        gl.bind_buffer(GL_ELEMENT_ARRAY_BUFFER, 0);
        gl.bind_vertex_array(0);
        gl.use_program(0);

        // Delete all resources.
        gl.delete_buffer(self.vertex_buffer_object);
        gl.delete_buffer(self.index_buffer_object);
        gl.delete_vertex_array(self.vertex_array_object);
        gl.delete_program(self.shader_program);
        gl.delete_shader(self.fragment_shader);
        gl.delete_shader(self.vertex_shader);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn on_open_gl_render(
        &self,
        gl: &GlInterface,
        _fb: i32,
        size: PixelSize,
        yaw: f32,
        pitch: f32,
        roll: f32,
        disco: f32,
    ) {
        gl.viewport(0, 0, size.width, size.height);
        gl.clear_depth(1.0);
        gl.disable(GL_CULL_FACE);
        gl.disable(GL_SCISSOR_TEST);
        gl.depth_func(GL_LESS);
        gl.depth_mask(1);

        gl.clear_color(0.0, 0.0, 0.0, 0.0);
        gl.clear(GL_COLOR_BUFFER_BIT | GL_DEPTH_BUFFER_BIT);
        gl.enable(GL_DEPTH_TEST);

        gl.bind_buffer(GL_ARRAY_BUFFER, self.vertex_buffer_object);
        gl.bind_buffer(GL_ELEMENT_ARRAY_BUFFER, self.index_buffer_object);
        gl.bind_vertex_array(self.vertex_array_object);
        gl.use_program(self.shader_program);
        Self::check_error(gl);
        // The aspect ratio is the quotient of the two integers, as in the original.
        let projection = Matrix4x4::create_perspective_field_of_view(
            (std::f64::consts::PI / 4.0) as f32,
            (size.width / size.height) as f32,
            0.01,
            1000.0,
        );

        let view = Matrix4x4::create_look_at(
            Vector3::new(25.0, 25.0, 25.0),
            Vector3::default(),
            Vector3::new(0.0, 1.0, 0.0),
        );
        let model = Matrix4x4::create_from_yaw_pitch_roll(yaw, pitch, roll);
        let model_loc = gl.get_uniform_location_string(self.shader_program, "uModel");
        let view_loc = gl.get_uniform_location_string(self.shader_program, "uView");
        let projection_loc = gl.get_uniform_location_string(self.shader_program, "uProjection");
        let max_y_loc = gl.get_uniform_location_string(self.shader_program, "uMaxY");
        let min_y_loc = gl.get_uniform_location_string(self.shader_program, "uMinY");
        let time_loc = gl.get_uniform_location_string(self.shader_program, "uTime");
        let disco_loc = gl.get_uniform_location_string(self.shader_program, "uDisco");
        // SAFETY: each pointer is to one matrix, sixteen singles in row order
        // (`Matrix4x4` is `#[repr(C)]`), which OpenGL reads during the call.
        unsafe {
            gl.uniform_matrix4fv(model_loc, 1, 0, &model as *const Matrix4x4 as *const c_void);
            gl.uniform_matrix4fv(view_loc, 1, 0, &view as *const Matrix4x4 as *const c_void);
            gl.uniform_matrix4fv(projection_loc, 1, 0, &projection as *const Matrix4x4 as *const c_void);
        }
        gl.uniform1f(max_y_loc, self.max_y);
        gl.uniform1f(min_y_loc, self.min_y);
        gl.uniform1f(time_loc, ST.elapsed().as_secs_f64() as f32);
        gl.uniform1f(disco_loc, disco);
        Self::check_error(gl);
        // SAFETY: a buffer is bound to `GL_ELEMENT_ARRAY_BUFFER`, so the pointer is an
        // offset into it; the buffer holds the indices counted.
        unsafe { gl.draw_elements(GL_TRIANGLES, self.indices.len() as i32, GL_UNSIGNED_SHORT, std::ptr::null()) };

        Self::check_error(gl);
    }
}

/// A block of the model: its length in bytes as a 32-bit integer, then its bytes.
fn read_block(sr: &mut impl Read) -> Vec<u8> {
    let mut length = [0u8; 4];
    sr.read_exact(&mut length).unwrap_or_else(|e| panic!("{e}"));
    let mut buf = vec![0u8; i32::from_le_bytes(length) as usize];
    sr.read_exact(&mut buf).unwrap_or_else(|e| panic!("{e}"));
    buf
}

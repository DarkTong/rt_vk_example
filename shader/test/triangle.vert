#version 450

layout (location = 0) in vec2 v_uv;
layout (location = 1) in vec3 v_color;

layout (location = 0) out vec3 f_color;
out gl_PerVertex {
    vec4 gl_Position;
};

void main()
{
    gl_Position = vec4(v_uv, 0.0, 1.0);
    f_color = v_color;
}

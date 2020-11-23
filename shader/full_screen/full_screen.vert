#version 450

layout(location = 0) in vec2 i_pos;

layout(location = 0) out vec2 o_uv;

void main()
{
    gl_Position = vec4(i_pos, 0, 1);
    o_uv = i_pos;
}

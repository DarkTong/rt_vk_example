#version 450

layout (location = 0) in vec2 i_uv;

layout (set = 0, binding = 0) uniform texture2D tex_screen;
layout (set = 0, binding = 0) uniform
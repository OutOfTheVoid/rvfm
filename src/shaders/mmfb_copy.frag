#version 450

layout(location=0) in vec2 copy_uv;

layout(location=0) out vec4 frag_color;

layout(set = 0, binding = 0) uniform texture2D copy_tex;
layout(set = 0, binding = 1) uniform sampler copy_sampler;

void main() {
	vec3 tex_color = texture(sampler2D(copy_tex, copy_sampler), copy_uv).rgb;
	frag_color = vec4(tex_color, 1.0);
}

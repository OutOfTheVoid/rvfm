#version 450

const vec2 positions[6] = vec2[6] (
	vec2(-1.0, +1.0),
	vec2(+1.0, +1.0),
	vec2(+1.0, -1.0),
	
	vec2(-1.0, +1.0),
	vec2(+1.0, -1.0),
	vec2(-1.0, -1.0)
);

layout (location = 0) out vec2 copy_uv;

void main() {
	vec2 position = positions[gl_VertexIndex];
	copy_uv = vec2(position.x + 0.5, position.y * -1 + 0.5);
	gl_Position = vec4(position, 0.0, 0.5);
}
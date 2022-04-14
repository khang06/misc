#version 330 core

layout (location = 0) in vec2 position;

uniform mat4 proj;

void main() {
    gl_Position = proj * vec4(position, 0.5, 1.0);
}
#version 300 es
precision highp float;
flat in vec4 fColor;
out vec4 outColor;

void main() {
    outColor = fColor;
}
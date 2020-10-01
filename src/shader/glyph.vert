#version 300 es
uniform mat3x2 uTransformationMatrix;
uniform vec2 uOrigin;
uniform vec2 uScale;
uniform sampler2D uGlyphDataTexture;

in vec2 aPosition;
in float aScale;
in vec4 aColor;
in int aGlyphDataIndex;
in int aGlyphNumVertices;

flat out vec4 fColor;

vec2 testPositions[6] = vec2[](
    vec2(-0.5, -0.5), vec2(0.5, -0.5), vec2(0.5, 0.5),
    vec2(-0.5, -0.5), vec2(-0.5, 0.5), vec2(0.5, 0.5)
);

vec4 getValueByIndexFromTexture(sampler2D tex, int index) {
    int texWidth = textureSize(tex, 0).x;
    int col = index % texWidth;
    int row = index / texWidth;
    return texelFetch(tex, ivec2(col, row), 0);
}

void main() {
    vec2 vertexPosition;
    if(gl_VertexID < aGlyphNumVertices) {
        int vertexIdx = aGlyphDataIndex + gl_VertexID;
        vertexPosition = getValueByIndexFromTexture(uGlyphDataTexture, vertexIdx).xy * aScale;
    } else {
        vertexPosition = vec2(0.0, 0.0); // degenerate vertex
    }
    vec2 transformedPosition = uOrigin +  uScale * aPosition;
    gl_Position = vec4(uTransformationMatrix * vec3(transformedPosition + vertexPosition, 1.0), 0.0, 1.0);
    fColor = aColor;
}
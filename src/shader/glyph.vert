#version 300 es
// layout (std140) uniform Transform {
    uniform mat3x2 uTransformationMatrix;
    uniform vec2 uOrigin;
    uniform vec2 uScale;
    uniform float uGlyphScale;
// };
uniform sampler2D uGlyphDataTexture;

in vec2 aPosition;
in float aScale;
in vec4 aStrokeColor;
in vec4 aFillColor;
in uvec4 aGlyphData; // (index, num_fill_vertices, num_stroke_vertices, _)

flat out vec4 fColor;

vec4 getValueByIndexFromTexture(sampler2D tex, uint index) {
    uint texWidth = uint(textureSize(tex, 0).x);
    int col = int(index % texWidth);
    int row = int(index / texWidth);
    return texelFetch(tex, ivec2(col, row), 0);
}

vec2 getVertexPosition() {
    uint glyphIndex = aGlyphData[0];
    uint numFillVertices = aGlyphData[1];
    uint numStrokeVertices = aGlyphData[2];
    uint vertexID = uint(gl_VertexID);
    if(vertexID < numFillVertices) {
        fColor = aFillColor;
    } else {
        fColor = aStrokeColor;
    }
    if(vertexID < numFillVertices + numStrokeVertices){
        return getValueByIndexFromTexture(uGlyphDataTexture, glyphIndex + vertexID).xy * aScale;
    }
    return vec2(0.0, 0.0); // degenerate vertex
}

void main() {
    vec2 vertexPosition = getVertexPosition();
    vec2 transformedPosition = uOrigin +  (vec2(1.0, -1.0) * uScale) * aPosition;
    gl_Position = vec4(uTransformationMatrix * vec3(transformedPosition + uGlyphScale * vertexPosition, 1.0), 0.0, 1.0);
}
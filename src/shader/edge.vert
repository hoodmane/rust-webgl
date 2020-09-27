#version 300 es
#define M_PI 3.1415926535897932384626433832795
#define ANGLE_RES 180 // should be same as ANGLE_RESOLUTION




// Note: this variant counts each pixel as 4 distinct floats.
float getValueByIndexFrom4ChannelTexture(sampler2D tex, int index){
    int texWidth = textureSize(tex, 0).x;
    int channel = index % 4;
    int texOffset = index / 4;
    int col = texOffset % texWidth;
    int row = texOffset / texWidth;
    return texelFetch(tex, ivec2(col, row), 0)[channel];
}

vec4 getValueByIndexFromTexture(sampler2D tex, int index) {
    int texWidth = textureSize(tex, 0).x;
    int col = index % texWidth;
    int row = index / texWidth;
    return texelFetch(tex, ivec2(col, row), 0);
}


float getGlyphBoundaryPoint(sampler2D tex, int glyph, float angle){
    int glyph_index = (int(angle / (2.0 * M_PI) * float(ANGLE_RES)) + ANGLE_RES) % ANGLE_RES;
    int total_index = ANGLE_RES * glyph + glyph_index;
    return getValueByIndexFrom4ChannelTexture(tex, total_index);
}

struct ArrowHeader {
    float tip_end;
    float back_end;
    float visual_tip_end;
    float visual_back_end;
    float line_end;
};

ArrowHeader readArrowHeader(sampler2D arrowHeaderTexture, int arrowHeaderIndex){
    float tip_end = getValueByIndexFrom4ChannelTexture(arrowHeaderTexture, arrowHeaderIndex);
    float back_end = getValueByIndexFrom4ChannelTexture(arrowHeaderTexture, arrowHeaderIndex + 1);
    float visual_tip_end = getValueByIndexFrom4ChannelTexture(arrowHeaderTexture, arrowHeaderIndex + 2);
    float visual_back_end = getValueByIndexFrom4ChannelTexture(arrowHeaderTexture, arrowHeaderIndex + 3);
    float line_end = getValueByIndexFrom4ChannelTexture(arrowHeaderTexture, arrowHeaderIndex + 4);
    return ArrowHeader(tip_end, back_end, visual_tip_end, visual_back_end, line_end);
}

vec2 getArrowVertex(sampler2D arrowVerticesTexture, int arrow_index, int vertex_index) {
    return getValueByIndexFromTexture(arrowVerticesTexture, arrow_index + vertex_index).xy;
}



uniform mat3x2 uTransformationMatrix;
uniform vec2 uOrigin;
uniform vec2 uScale;
uniform sampler2D uGlyphDataTexture;
uniform sampler2D uArrowHeaderTexture;
uniform sampler2D uArrowPathTexture;
// uniform sampler2D uArrowTipPathTexture;

in vec4 aColor;
in vec2 aStartPosition;
in vec2 aEndPosition;
in int aStartGlyph;
in int aEndGlyph;
in float aStartGlyphScale;
in float aEndGlyphScale;

in int aStartArrowNumVertices;
in int aStartArrowHeaderIndex;
in int aStartArrowVerticesIndex;

in int aEndArrowNumVertices;
in int aEndArrowHeaderIndex;
in int aEndArrowVerticesIndex;

flat out vec4 fColor;


ivec2 vertexIndexes[6] = ivec2[](
    ivec2(0, 0), ivec2(0, 1), ivec2(1, 0),
    ivec2(0, 1), ivec2(1, 0), ivec2(1, 1)
);

vec2 testPositions[3] = vec2[](
    vec2(0.8, 1.34641), vec2(2.2, 1.34641), vec2(1.5, 0.133975)
);


void main() {
    vec2 transformedStart = uOrigin +  uScale * aStartPosition;
    vec2 transformedEnd = uOrigin +  uScale * aEndPosition;

    vec2 displacement = normalize(transformedEnd - transformedStart);
    float angle = atan(displacement.y, displacement.x);
    float startOffset = aStartGlyphScale * getGlyphBoundaryPoint(uGlyphDataTexture, aStartGlyph, angle);
    float endOffset = aEndGlyphScale * getGlyphBoundaryPoint(uGlyphDataTexture, aEndGlyph, angle + M_PI);

    vec2 startVec = transformedStart + startOffset * displacement;
    vec2 endVec = transformedEnd - endOffset * displacement;

    vec2 normal = vec2(-displacement.y, displacement.x);

    vec2 position;
    if(gl_VertexID < 6){
        // Line
        ivec2 vertexIndex = vertexIndexes[gl_VertexID];

        if(vertexIndex.x == 1){
            normal = - normal;
        }

        if(vertexIndex.y == 0){
            position = startVec + normal;
        } else {
            position = endVec + normal;
        }
    } else if(gl_VertexID < 6 + aStartArrowNumVertices) {
        // Start arrow
        int vertex_index = gl_VertexID - 6;
        mat2 rotationMatrix = mat2(displacement, normal);
        position = startVec - rotationMatrix * getArrowVertex(uArrowPathTexture, aStartArrowVerticesIndex, vertex_index).xy;
    } else if(gl_VertexID < 6 + aStartArrowNumVertices + aEndArrowNumVertices) {
        // End arrow
        int vertex_index = gl_VertexID - 6 - aStartArrowNumVertices;
        mat2 rotationMatrix = mat2(displacement, normal);
        position = endVec + rotationMatrix * getArrowVertex(uArrowPathTexture, aEndArrowVerticesIndex, vertex_index).xy;
    } else {
        // Extra throw-away vertices
        position = vec2(0.0, 0.0);
    }

    gl_Position = vec4(uTransformationMatrix * vec3(position, 1.0), 0.0, 1.0);
    fColor = aColor;
    if(aStartArrowNumVertices == 0){
        fColor.g = 1.0;
    }
}
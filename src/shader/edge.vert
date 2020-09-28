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

struct Arrow {
    int numVertices; 
    int headerIndex;
    int verticesIndex;    
    float tip_end;
    float back_end;
    float visual_tip_end;
    float visual_back_end;
    float line_end;
};

Arrow makeArrow(sampler2D arrowHeaderTexture, ivec3 arrow){
    int numVertices = arrow[0];
    int headerIndex = arrow[1];
    int verticesIndex = arrow[2];
    float tip_end = getValueByIndexFrom4ChannelTexture(arrowHeaderTexture, headerIndex);
    float back_end = getValueByIndexFrom4ChannelTexture(arrowHeaderTexture, headerIndex + 1);
    float visual_tip_end = getValueByIndexFrom4ChannelTexture(arrowHeaderTexture, headerIndex + 2);
    float visual_back_end = getValueByIndexFrom4ChannelTexture(arrowHeaderTexture, headerIndex + 3);
    float line_end = getValueByIndexFrom4ChannelTexture(arrowHeaderTexture, headerIndex + 4);
    return Arrow(numVertices, headerIndex, verticesIndex, tip_end, back_end, visual_tip_end, visual_back_end, line_end);
}

vec2 getArrowVertex(sampler2D arrowVerticesTexture, Arrow arrow, int vertex_index) {
    return getValueByIndexFromTexture(arrowVerticesTexture, arrow.verticesIndex + vertex_index).xy;
}



uniform mat3x2 uTransformationMatrix;
uniform vec2 uOrigin;
uniform vec2 uScale;
uniform sampler2D uGlyphDataTexture;
uniform sampler2D uArrowHeaderTexture;
uniform sampler2D uArrowPathTexture;


in vec4 aColor;
in vec4 aPositions; // (start_position, end_position)
in vec2 aGlyphScales; // (start_glyph_scale, end_glyph_scale)
in ivec4 aStart; // (startGlyph, vec3 startArrow = (NumVertices, HeaderIndex, VerticesIndex) ) 
in ivec4 aEnd; // (endGlyph, vec3 endArrow = (NumVertices, HeaderIndex, VerticesIndex) )

flat out vec4 fColor;


ivec2 vertexIndexes[6] = ivec2[](
    ivec2(0, 0), ivec2(0, 1), ivec2(1, 0),
    ivec2(0, 1), ivec2(1, 0), ivec2(1, 1)
);

vec2 testPositions[3] = vec2[](
    vec2(0.8, 1.34641), vec2(2.2, 1.34641), vec2(1.5, 0.133975)
);


void main() {
    vec2 startPosition = aPositions.xy;
    vec2 endPosition = aPositions.zw;
    vec2 transformedStart = uOrigin +  uScale * startPosition;
    vec2 transformedEnd = uOrigin + uScale * endPosition;

    int startGlyph = aStart.x;
    int endGlyph = aEnd.x;

    ivec3 startArrowData = aStart.yzw;
    ivec3 endArrowData = aEnd.yzw;

    vec2 displacement = normalize(transformedEnd - transformedStart);
    float angle = atan(displacement.y, displacement.x);
    float startOffset = aGlyphScales.x * getGlyphBoundaryPoint(uGlyphDataTexture, startGlyph, angle);
    float endOffset = aGlyphScales.y * getGlyphBoundaryPoint(uGlyphDataTexture, endGlyph, angle + M_PI);

    vec2 startVec = transformedStart + startOffset * displacement;
    vec2 endVec = transformedEnd - endOffset * displacement;

    Arrow startArrow = makeArrow(uArrowHeaderTexture, startArrowData);
    Arrow endArrow = makeArrow(uArrowHeaderTexture, endArrowData);

    vec2 adjustedStartVec = startVec - startArrow.line_end * displacement;
    vec2 adjustedEndVec = endVec + endArrow.line_end * displacement;

    vec2 normal = vec2(-displacement.y, displacement.x);

    vec2 position;
    if(gl_VertexID < 6){
        // Line
        ivec2 vertexIndex = vertexIndexes[gl_VertexID];

        if(vertexIndex.x == 1){
            normal = - normal;
        }

        if(vertexIndex.y == 0){
            position = adjustedStartVec + normal;
        } else {
            position = adjustedEndVec + normal;
        }
    } else if(gl_VertexID < 6 + startArrow.numVertices) {
        // Start arrow
        int vertex_index = gl_VertexID - 6;
        mat2 rotationMatrix = mat2(displacement, normal);
        position = startVec - rotationMatrix * getArrowVertex(uArrowPathTexture, startArrow, vertex_index).xy;
    } else if(gl_VertexID < 6 + startArrow.numVertices + endArrow.numVertices) {
        // End arrow
        int vertex_index = gl_VertexID - 6 - startArrow.numVertices;
        mat2 rotationMatrix = mat2(displacement, normal);
        position = endVec + rotationMatrix * getArrowVertex(uArrowPathTexture, endArrow, vertex_index).xy;
    } else {
        // Extra throw-away vertices
        position = vec2(0.0, 0.0);
    }

    gl_Position = vec4(uTransformationMatrix * vec3(position, 1.0), 0.0, 1.0);
    fColor = aColor;
}
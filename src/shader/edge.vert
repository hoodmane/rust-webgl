#version 300 es
#define M_PI 3.1415926535897932384626433832795
#define ANGLE_RES 180 // should be same as ANGLE_RESOLUTION

// this variant counts each pixel as 4 distinct floats.
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




uniform mat3x2 uTransformationMatrix;
uniform vec2 uOrigin;
uniform vec2 uScale;
uniform sampler2D uGlyphBoundaryTexture;
uniform sampler2D uArrowHeaderTexture;
uniform sampler2D uArrowPathTexture;


in vec4 aColor;
in vec4 aStartPosition; // (start_position, start_tangent)
in vec4 aEndPosition; // (end_position, end_tangent)
in vec4 aGlyphScales_and_2SinAngle; // (start_glyph_scale, end_glyph_scale)
in ivec4 aStart; // (startGlyph, vec3 startArrow = (NumVertices, HeaderIndex, VerticesIndex) ) 
in ivec4 aEnd; // (endGlyph, vec3 endArrow = (NumVertices, HeaderIndex, VerticesIndex) )

flat out vec4 fColor;

flat out float fCurvature;
flat out vec2 fP0;
flat out vec2 fN0;
flat out float fHalfThickness;
out vec2 vPosition;


ivec2 vertexIndexes[6] = ivec2[](
    ivec2(0, 0), ivec2(0, 1), ivec2(1, 0),
    ivec2(0, 1), ivec2(1, 0), ivec2(1, 1)
);

vec2 testPositions[3] = vec2[](
    vec2(0.8, 1.34641), vec2(2.2, 1.34641), vec2(1.5, 0.133975)
);



vec2 transformPos(vec2 pos){
    return uOrigin + uScale * pos;
}

vec2 transformTan(vec2 tangent){
    return normalize(uScale * tangent);
}

vec4 transformVec(vec4 pos_tan) {
    return vec4(transformPos(pos_tan.xy), transformTan(pos_tan.zw));
}


float glyphBoundaryPoint(int glyph, float angle){
    int glyph_index = (int(angle / (2.0 * M_PI) * float(ANGLE_RES)) + ANGLE_RES) % ANGLE_RES;
    int total_index = ANGLE_RES * glyph + glyph_index;
    return getValueByIndexFrom4ChannelTexture(uGlyphBoundaryTexture, total_index);
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

Arrow makeArrow(ivec3 arrow){
    int numVertices = arrow[0];
    int headerIndex = arrow[1];
    int verticesIndex = arrow[2];
    float tip_end = getValueByIndexFrom4ChannelTexture(uArrowHeaderTexture, headerIndex);
    float back_end = getValueByIndexFrom4ChannelTexture(uArrowHeaderTexture, headerIndex + 1);
    float visual_tip_end = getValueByIndexFrom4ChannelTexture(uArrowHeaderTexture, headerIndex + 2);
    float visual_back_end = getValueByIndexFrom4ChannelTexture(uArrowHeaderTexture, headerIndex + 3);
    float line_end = getValueByIndexFrom4ChannelTexture(uArrowHeaderTexture, headerIndex + 4);
    return Arrow(numVertices, headerIndex, verticesIndex, tip_end, back_end, visual_tip_end, visual_back_end, line_end);
}

int arrowNumVertices(ivec3 arrow){
    return arrow[0];
}


float arrowLineEnd(ivec3 arrow){
    int headerIndex = arrow[1];
    return getValueByIndexFrom4ChannelTexture(uArrowHeaderTexture, headerIndex + 4);
}


vec2 getArrowVertex(ivec3 arrow, int vertexIndex) {
    int verticesIndex = arrow[2];
    return getValueByIndexFromTexture(uArrowPathTexture, verticesIndex + vertexIndex).xy;
}




// This is the special case of circleOffset when position = (0, 0) and tangent = (1, 0). 
// In that case the circle looks like the graph of r = sin(theta).
// Needs: epsilon < dist * abs(curvature) / 2 < 1 - epsilon (upper bound comes from dist < diameter).
// curvature -- curvature of circle (1/radius). If curvature > 0 it curves leftward, if curvature < 0 it curves rightward.
// dist -- distance to move along circle
// direction -- does circle curve to the left or to the right of the tangent vector.
vec4 circleOffsetHelper(float curvature, float dist) {
    // If dist == 0, tangent, vector we want to normalize to get tangent is (0, 0).
    // Thus, it's necesary to special case distance == 0. (What about distance small?)
    if(dist == 0.0){
        return vec4(0.0, 0.0, 1.0, 0.0);
    }
    // ??
    // if(dist < epsilon){
    //     return vec4(epsilon, 0.0, 1.0, 0.0);
    // }
    float x = dist;
    float C = curvature;
    float cx_over_2 = C*x/2.0;
    float om_cx_over_2_sq = 1.0 - cx_over_2 * cx_over_2;
    // position = x(sqrt(1 - (Cx/2)^2), Cx/2)
    // tangent = position * (4(1 - (Cx/2)^2) - 2, 4(1 - (Cx/2)^2))
    vec2 position = x * vec2(sqrt(om_cx_over_2_sq), cx_over_2);
    vec2 tangent_factor = (4.0 * om_cx_over_2_sq) * vec2(1.0, 1.0) - vec2(2.0, 0.0);
    // tangent_factor * position is the double angle formula applied to position. If the double
    vec2 tangent = normalize(tangent_factor * position);
    return vec4(position, tangent);
}

// There is a unique circle through pos with tangent vector tan at pos and given curvature that curves to the left.
// start_pos_tan -- start position and tangent.
// curvature -- 1/radius, with sign: if curvature > 0 it curves leftward, if curvature < 0 it curves rightward.
// dist -- distance to move along circle.
// Needs: epsilon < dist * abs(curvature) / 2 < 1 - epsilon (upper bound comes from dist < diameter).
vec4 circleOffset(vec4 start_pos_tan, float curvature, float dist){
    vec2 start_pos = start_pos_tan.xy;
    vec2 start_tan = start_pos_tan.zw;
    // Save time if we are drawing lines (longer method will give same result).
    if(curvature == 0.0) {
        return vec4(start_pos + start_tan * dist, start_tan);
    }
    vec4 helper_pos_tan = circleOffsetHelper(curvature, dist);
    vec2 helper_pos = helper_pos_tan.xy;
    vec2 helper_tan = helper_pos_tan.zw;
    mat2 rotation = mat2(start_tan, start_tan.yx * vec2(-1.0, 1.0));
    vec2 result_pos = start_pos + rotation * helper_pos;
    vec2 result_tan = rotation * helper_tan;
    return vec4(result_pos, result_tan);
}


float glyphOffsetLinear(int glyph, float scale, float angle){
    return scale * glyphBoundaryPoint(glyph, angle);
}

float glyphOffsetCurvedHelper(int glyph, float scale, vec2 tangent){
    float angle = atan(tangent.y, tangent.x);
    return scale * glyphBoundaryPoint(glyph, angle);
}

vec4 glyphOffsetCurved(int glyph, float scale, vec2 position, vec2 tangent, float curvature){
    float offset = glyphOffsetCurvedHelper(glyph, scale, tangent);
    vec4 pos_tan = circleOffset(vec4(position, tangent), curvature, offset);
    // Try again with more accurate angle.
    vec2 secant = pos_tan.zw;
    offset = glyphOffsetCurvedHelper(glyph, scale, secant);
    return circleOffset(vec4(position, tangent), curvature, offset);
}


vec2 vertexPositionLinear(){
    vec2 startPos = transformPos(aStartPosition.xy);
    vec2 endPos = transformPos(aEndPosition.xy);
    vec2 tangent = normalize(endPos - startPos);
    vec2 normal = tangent.yx * vec2(-1.0, 1.0);
    float angle = atan(tangent.y, tangent.x);
    int startGlyph = aStart.x;
    int endGlyph = aEnd.x;
    float startGlyphScale = aGlyphScales_and_2SinAngle.x;
    float endGlyphScale = aGlyphScales_and_2SinAngle.y;
    startPos += tangent * glyphOffsetLinear(startGlyph, startGlyphScale, angle);
    endPos -= tangent * glyphOffsetLinear(endGlyph, endGlyphScale, angle + M_PI);

    ivec3 startArrow = aStart.yzw;
    ivec3 endArrow = aEnd.yzw;

    // Arrow startArrow = makeArrow(uArrowHeaderTexture, startArrowData);
    // Arrow endArrow = makeArrow(uArrowHeaderTexture, endArrowData);

    int vertexID = gl_VertexID;
    if(vertexID < 6){
        startPos -= tangent * arrowLineEnd(startArrow);
        endPos += tangent * arrowLineEnd(endArrow);

        ivec2 vertexIndex = vertexIndexes[gl_VertexID];
        if(vertexIndex.x == 1){
            normal = - normal;
        }
        if(vertexIndex.y == 0){
            return startPos + normal;
        } else {
            return endPos + normal;
        }
    }
    vertexID -= 6;
    
    mat2 rotationMatrix = mat2(tangent, normal);
    // Start arrow
    if(vertexID < arrowNumVertices(startArrow)) {
        return startPos - rotationMatrix * getArrowVertex(startArrow, vertexID);
    } 
    vertexID -= arrowNumVertices(startArrow);
    
    // End arrow
    if(vertexID < arrowNumVertices(endArrow)) {
        return endPos + rotationMatrix * getArrowVertex(endArrow, vertexID).xy;
    }
    vertexID -= arrowNumVertices(endArrow);
    
    // Extra throw-away vertices
    return vec2(0.0, 0.0);
}


vec2 vertexPositionCurved(){
    return vec2(0.0, 0.0);
}

void main() {
    float double_sin_angle = aGlyphScales_and_2SinAngle.z;
    vec2 position;
    if(double_sin_angle == 0.0){
        position = vertexPositionLinear();
    } else {
        position = vertexPositionCurved();
    }
    gl_Position = vec4(uTransformationMatrix * vec3(position, 1.0), 0.0, 1.0);
    fColor = aColor;
}
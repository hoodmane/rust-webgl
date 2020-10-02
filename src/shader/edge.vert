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



// layout (std140) uniform Transform {
    uniform mat3x2 uTransformationMatrix;
    uniform vec2 uOrigin;
    uniform vec2 uScale;
// };

uniform sampler2D uGlyphBoundaryTexture;
uniform sampler2D uArrowHeaderTexture;
uniform sampler2D uArrowPathTexture;


in vec4 aColor;
in vec4 aPositions; // (start_position, end_position)
in vec4 aGlyphScales_angle_thickness; // (start_glyph_scale, end_glyph_scale, angle, thickness)
in ivec4 aStart; // (startGlyph, vec3 startArrow = (NumVertices, HeaderIndex, VerticesIndex) ) 
in ivec4 aEnd; // (endGlyph, vec3 endArrow = (NumVertices, HeaderIndex, VerticesIndex) )

out vec4 fColor;

flat out float fCurvature;
flat out vec2 fP0;
flat out vec2 fN0;
flat out float fHalfThickness;
out vec2 vPosition;

vec2 transformPos(vec2 pos){
    return uOrigin + uScale * pos;
}

vec4 reverseTangent(vec4 pos_tan){
    return pos_tan * vec4(1.0, 1.0, -1.0, -1.0);
}

vec2 normalVector(vec2 direction){
    return direction.yx * vec2(-1.0, 1.0);
}

mat2 rotationMatrix(vec2 direction){
    return mat2(direction, normalVector(direction));
}

float glyphBoundaryPoint(int glyph, float angle){
    int glyph_index = (int(angle / (2.0 * M_PI) * float(ANGLE_RES)) + ANGLE_RES) % ANGLE_RES;
    int total_index = ANGLE_RES * glyph + glyph_index;
    return getValueByIndexFrom4ChannelTexture(uGlyphBoundaryTexture, total_index);
}


int arrowNumVertices(ivec3 arrow){
    return arrow[0];
}

vec2 arrowEnds(ivec3 arrow){
    int headerIndex = arrow[1];
    float tip_end = getValueByIndexFrom4ChannelTexture(uArrowHeaderTexture, headerIndex);
    float back_end = getValueByIndexFrom4ChannelTexture(uArrowHeaderTexture, headerIndex + 1);
    return vec2(tip_end, back_end);
}

vec2 arrowVisualEnds(ivec3 arrow){
    int headerIndex = arrow[1];
    float visual_tip_end = getValueByIndexFrom4ChannelTexture(uArrowHeaderTexture, headerIndex + 2);
    float visual_back_end = getValueByIndexFrom4ChannelTexture(uArrowHeaderTexture, headerIndex + 3);
    return vec2(visual_tip_end, visual_back_end);
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
    float C = -curvature;
    float cx_over_2 = C*x/2.0;
    float om_cx_over_2_sq = 1.0 - cx_over_2 * cx_over_2;
    // position = x(sqrt(1 - (Cx/2)^2), Cx/2)
    // tangent = position * (4(1 - (Cx/2)^2) - 2, 4(1 - (Cx/2)^2))
    vec2 position = x * vec2(sqrt(om_cx_over_2_sq), cx_over_2);
    vec2 tangent_factor = (4.0 * om_cx_over_2_sq) * vec2(1.0, 1.0) - vec2(2.0, 0.0);
    // tangent_factor * position is the double angle formula applied to position. If the double
    vec2 tangent = normalize(tangent_factor * position);
    if(dist < 0.0){
        tangent *= -1.0;
    }
    return vec4(position, tangent);
}

// There is a unique circle through pos with tangent vector tan at pos and given curvature that curves to the left.
// start_pos_tan -- start position and tangent.
// curvature -- 1/radius, with sign: if curvature > 0 it curves leftward, if curvature < 0 it curves rightward.
// dist -- secand length along circle.
// Needs: epsilon < dist * abs(curvature) / 2 < 1 - epsilon (upper bound comes from dist < diameter).
vec4 circleOffset(vec4 start_pos_tan, float curvature, float dist){
    vec2 start_pos = start_pos_tan.xy;
    vec2 start_tan = start_pos_tan.zw;
    vec4 helper_pos_tan = circleOffsetHelper(curvature, dist);
    vec2 helper_pos = helper_pos_tan.xy;
    vec2 helper_tan = helper_pos_tan.zw;
    mat2 rotation = rotationMatrix(start_tan);
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

vec4 glyphOffsetCurved(int glyph, float scale, float extra, vec4 pos_tan, float curvature){
    float offset = glyphOffsetCurvedHelper(glyph, scale, pos_tan.zw) + extra;
    // return circleOffset(pos_tan, curvature, offset);
    vec2 secant = circleOffset(pos_tan, curvature, offset).zw;
    // Try again with more accurate angle.
    offset = glyphOffsetCurvedHelper(glyph, scale, secant) + extra;
    return circleOffset(pos_tan, curvature, offset);
}


vec2 vertexPositionLinear(){
    vec2 startPos = transformPos(aPositions.xy);
    vec2 endPos = transformPos(aPositions.zw);
    vec2 tangent = normalize(endPos - startPos);
    float angle = atan(tangent.y, tangent.x);

    int startGlyph = aStart.x;
    int endGlyph = aEnd.x;
    float startGlyphScale = aGlyphScales_angle_thickness.x;
    float endGlyphScale = aGlyphScales_angle_thickness.y;
    float thickness = aGlyphScales_angle_thickness.w;
    startPos += tangent * glyphOffsetLinear(startGlyph, startGlyphScale, angle);
    endPos -= tangent * glyphOffsetLinear(endGlyph, endGlyphScale, angle + M_PI);

    ivec3 startArrow = aStart.yzw;
    ivec3 endArrow = aEnd.yzw;

    int vertexID = gl_VertexID;
    if(vertexID < 6){
        startPos -= tangent * arrowLineEnd(startArrow);
        endPos += tangent * arrowLineEnd(endArrow);

        int vertexIndex = (vertexID/3) + (vertexID % 3);
        vec2 normal = normalVector(tangent);
        if(vertexIndex % 2 == 1){
            normal = - normal;
        }
        if(vertexIndex/2 == 0){
            return startPos + thickness * normal;
        } else {
            return endPos + thickness * normal;
        }
    }
    vertexID -= 6;
    
    mat2 rotationMatrix = rotationMatrix(tangent);
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


vec2 positionCurvedArrrow(ivec3 arrow, int glyph, float glyphScale, vec4 posTan, float curvature, int vertexID){
    vec2 ends = arrowVisualEnds(arrow);
    float tipEnd = ends[0];
    float backEnd = ends[1];
    vec4 tipEndPosTan = glyphOffsetCurved(glyph, glyphScale, tipEnd, posTan, curvature);
    vec4 backEndPosTan = circleOffset(tipEndPosTan, curvature, -tipEnd + backEnd);
    vec2 secant = normalize(tipEndPosTan.xy - backEndPosTan.xy);
    mat2 rotationMatrix = rotationMatrix(secant);
    return tipEndPosTan.xy - rotationMatrix * getArrowVertex(arrow, vertexID);
}

vec2 vertexPositionCurved(){
    vec2 startPos = transformPos(aPositions.xy);
    vec2 endPos = transformPos(aPositions.zw);
    vec2 displacement = endPos.xy - startPos.xy;
    float angle = aGlyphScales_angle_thickness.z;
    float segment_angle = atan(displacement.y, displacement.x);
    float start_tangent_angle = segment_angle + angle;
    float end_tangent_angle = segment_angle - angle;
    vec2 start_tangent = vec2(cos(start_tangent_angle), sin(start_tangent_angle));
    vec2 end_tangent = vec2(cos(end_tangent_angle), sin(end_tangent_angle));
    vec4 startPosTan = vec4(startPos, start_tangent);
    vec4 endPosTan = vec4(endPos, end_tangent);


    bool curvesLeft = angle < 0.0; // aGlyphScales_angle_thickness.z < 0.0;
    float thickness = aGlyphScales_angle_thickness.w;
    float curvature = 2.0 * sin(angle) / length(displacement);
    int startGlyph = aStart.x;
    int endGlyph = aEnd.x;
    float startGlyphScale = aGlyphScales_angle_thickness.x;
    float endGlyphScale = aGlyphScales_angle_thickness.y;
    ivec3 startArrow = aStart.yzw;
    ivec3 endArrow = aEnd.yzw;

    int vertexID = gl_VertexID;

    if(vertexID < 12){
        fCurvature = curvature;
        fP0 = startPosTan.xy;
        fN0 = normalVector(startPosTan.zw);
        fHalfThickness = thickness;

        vec4 origStartPosTan = startPosTan;
        vec4 origEndPosTan = endPosTan;
        startPosTan = glyphOffsetCurved(startGlyph, startGlyphScale, -arrowLineEnd(startArrow), startPosTan, curvature);
        endPosTan = reverseTangent(glyphOffsetCurved(endGlyph, endGlyphScale, -arrowLineEnd(endArrow), reverseTangent(endPosTan), -curvature));
        int vidx = (vertexID/3) + (vertexID % 3);
        switch(vertexID/3){
            case 0:
                fColor = vec4(0.0, 0.0, 0.0, 0.3);
                break;
            case 1:
                fColor = vec4(1.0, 0.0, 0.0, 0.3);
                break;
            case 2:
                fColor = vec4(0.0, 1.0, 0.0, 0.3);
                break;
            case 3:
                fColor = vec4(0.0, 0.0, 1.0, 0.3);
                break;
        }
        bool inside = (vidx + 1 - vertexID/6) % 2 == 0;
        inside = inside != curvesLeft;
        int angle_idx = vidx / 2;
        vec2 displacement = (origEndPosTan.xy - origStartPosTan.xy);
        float displacement_length = length(displacement);
        vec2 midNormal = normalVector(normalize(displacement));
        vec2 midPos = (origStartPosTan.xy + origEndPosTan.xy) / 2.0 + (displacement_length/2.0 * tan(angle/2.0)) * midNormal;
        // vec2 midPos = circleOffset(origStartPosTan, curvature, displacement_length/2.0/cos(angle/2.0)).xy;

        vec2 pos;
        vec2 normal;
        switch(angle_idx){
            case 0:
                pos = startPosTan.xy;
                normal = normalVector(startPosTan.zw);
                break;
            case 1:
                pos = midPos;
                normal = midNormal;
                break;
            case 2:
                pos = endPosTan.xy;
                normal = normalVector(endPosTan.zw);
                break;
        }

        float thickness_scale = 2.0;
        float offset;
        if(inside){
            offset = -thickness_scale * thickness;        
        } else {
            // vec2 quarterNormal = normalize(normalVector(origStartPosTan.zw) + midNormal);
            // float magnitude = length(midPos - origStartPosTan.xy)/2.0 * abs(tan(angle/4.0));
            // vec2 v = (magnitude + thickness) * quarterNormal;
            // offset = dot(v, v)/dot(v, midNormal) + thickness_scale * thickness;
            float magnitude = length(midPos - origStartPosTan.xy)/2.0 * abs(tan(angle/4.0))/cos(angle/2.0);
            offset = magnitude + thickness_scale * thickness;
        }
        if(curvesLeft){
            offset = -offset;
        }
        pos += offset * normal;
        vPosition = pos;
        return pos;
    }
    vertexID -= 12;

    // Start arrow
    if(vertexID < arrowNumVertices(startArrow)) {
        return positionCurvedArrrow(startArrow, startGlyph, startGlyphScale, startPosTan, curvature, vertexID);
    } 
    vertexID -= arrowNumVertices(startArrow);
    
    // End arrow
    if(vertexID < arrowNumVertices(endArrow)) {
        return positionCurvedArrrow(endArrow, endGlyph, endGlyphScale, reverseTangent(endPosTan), -curvature, vertexID);
    }
    vertexID -= arrowNumVertices(endArrow);
    
    // Extra throw-away vertices
    return vec2(0.0, 0.0);
}

void main() {
    fColor = aColor;
    float angle = aGlyphScales_angle_thickness.z;
    vec2 position;
    if(angle == 0.0){
        position = vertexPositionLinear();
    } else {
        position = vertexPositionCurved();
    }
    gl_Position = vec4(uTransformationMatrix * vec3(position, 1.0), 0.0, 1.0);
}
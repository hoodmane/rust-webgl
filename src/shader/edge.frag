#version 300 es
precision highp float;

uniform sampler2D uDashPatterns;

in vec4 fColor;
flat in float fCurvature;
flat in vec2 fCenter;
flat in float fInitialAngle;
flat in vec2 fP0;
flat in vec2 fN0;
flat in float fHalfThickness;
flat in ivec4 fDashPattern;
in vec2 vPosition;
out vec4 outColor;

// returns 0 if gradient << compValue, 1 if gradient >> compValue,
// if gradient ~ compValue linearly interpolates a single pixel
// https://www.ronja-tutorials.com/2019/11/29/fwidth.html#a-better-step
float aaStep(float compValue, float gradient){
    float halfChange = fwidth(gradient) / 2.0;
    //base the range of the inverse lerp on the change over one pixel
    float lowerEdge = compValue - halfChange;
    float upperEdge = compValue + halfChange;
    //do the inverse interpolation
    float stepped = (gradient - lowerEdge) / (upperEdge - lowerEdge);
    stepped = clamp(stepped, 0.0, 1.0);
    return stepped;
}

float circleConstraint(float ab_dot_ab, float ab_dot_n, float epsilon, float curvature){
    float numerator = -2.0 * (ab_dot_n + epsilon);
    float denominator = ab_dot_ab + 2.0 * epsilon * ab_dot_n + epsilon * epsilon;
    float C_e = curvature * epsilon;
    float C_2_e = curvature * C_e;
    float comparison = curvature + C_2_e + C_2_e * C_e;
    return numerator/denominator - comparison;
}

float getDashOpacity(float arcLength){
    int dashLength = fDashPattern.x;
    int dashIndex = fDashPattern.y;
    int dashOffset = fDashPattern.z;
    ivec2 texSize = textureSize(uDashPatterns, 0);

    float xCoord = mod(arcLength, float(dashLength)) / float(texSize.x);
    float yCoord = float(dashIndex) / float(texSize.y);
    return texture(uDashPatterns, vec2(xCoord, yCoord)).r;
}

void main() {
    bool dashPatternQ = fDashPattern.x != 0;
    outColor = fColor;
    if(dashPatternQ){
        float arcLength;
        if(abs(fCurvature) > 0.0001){
            vec2 offsetFromCenter = vPosition - fCenter;
            float angle = atan(offsetFromCenter.y, offsetFromCenter.x) - fInitialAngle;
            arcLength = angle / fCurvature;
        } else {
            vec2 T0 = fN0.yx * vec2(1.0, -1.0);
            arcLength = dot(vPosition - fP0, T0);
        }
        // TODO: Now sample from dash pattern texture
        outColor.a *= getDashOpacity(arcLength);
    }

    if(fCurvature == 0.0){
        outColor.rgb *= outColor.a;
        return;
    }
    vec2 ab = vPosition - fP0;
    float ab_dot_n = dot(ab, fN0);
    float ab_dot_ab = dot(ab, ab);
    float inner_bound =   circleConstraint(ab_dot_ab, ab_dot_n, - fHalfThickness, fCurvature);
    float outer_bound = - circleConstraint(ab_dot_ab, ab_dot_n,   fHalfThickness, fCurvature);
    float bound = min(inner_bound, outer_bound);
    float alpha = aaStep(0.0, bound);
    outColor.a *= alpha;
    outColor.rgb *= outColor.a;
    // if(alpha != 0.0) {
    //     outColor = vec4(0.6, 0.0, 1.0, alpha);
    // }
}
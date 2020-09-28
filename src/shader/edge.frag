#version 300 es
precision highp float;
flat in vec4 fColor;
flat in float fCurvature;
flat in vec2 fP0;
flat in vec2 fN0;
flat in float fHalfThickness;
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

void main() {
    bool weHaveDashPattern = false;
    outColor = fColor;
    return;
    if(fCurvature == 0.0){
        if(weHaveDashPattern){
            float arc_length = length(vPosition - fP0);
            // Sample from dash pattern texture
        }
        return;
    }
    vec2 ab = vPosition - fP0;
    float ab_dot_n = dot(ab, fN0);
    float ab_dot_ab = dot(ab, ab);
    if(weHaveDashPattern){
        float ab_length = sqrt(ab_dot_ab);
        float sin_theta = ab_dot_n / ab_length;
        float theta = asin(sin_theta);
        float arc_length = (theta/sin_theta) * ab_length;
        // TODO: Now sample from dash pattern texture
    }
    float inner_bound =   circleConstraint(ab_dot_ab, ab_dot_n, - fHalfThickness, fCurvature);
    float outer_bound = - circleConstraint(ab_dot_ab, ab_dot_n,   fHalfThickness, fCurvature);
    float bound = min(inner_bound, outer_bound);
    float alpha = aaStep(0.0, bound);
    outColor.a *= alpha;
    if(alpha == 0.0) {
        discard;
    }
}
// Bevel, refracted highlights and dispersion adapted from the portfolio.
// The Windows compositor supplies the scene through the transparent face.
RWTexture2D<unorm float4> panel : register(u0);

cbuffer Surface : register(b0) {
    float4 lens;     // width, height, corner radius, bevel width in DIPs
    float4 optics;   // refraction, distortion, dispersion, light mode
    float4 pointer;  // cursor position normalized to the panel
};

[numthreads(8, 8, 1)]
void main(uint3 thread : SV_DispatchThreadID) {
    uint width, height;
    panel.GetDimensions(width, height);
    if (thread.x >= width || thread.y >= height) return;

    float2 halfSize = lens.xy * 0.5;
    float2 p = ((thread.xy + 0.5) / float2(width, height) - 0.5) * lens.xy;
    float2 q = abs(p) - halfSize + lens.z;
    float2 corner = max(q, 0.0);
    float cornerDistance = length(corner);
    float edge = cornerDistance + min(max(q.x, q.y), 0.0) - lens.z;
    if (edge >= 0.5 || edge < -max(3.0, lens.w)) {
        panel[thread.xy] = 0;
        return;
    }

    float horizontal = smoothstep(-0.85, 0.85, q.x - q.y);
    float2 flatNormal = normalize(float2(horizontal, 1.0 - horizontal));
    float2 edgeNormal = sign(p) * (cornerDistance > 0.0001 ? corner / cornerDistance : flatNormal);
    float bevel = 1.0 - smoothstep(0.0, max(lens.w, 0.001), -edge);
    float edgeHeight = sqrt(max(0.065, 1.0 - bevel * bevel));
    float2 flex = p / halfSize * (0.10 * optics.y);
    float3 normal = normalize(float3(edgeNormal * bevel * 0.97 + flex, edgeHeight));
    float3 view = normalize(float3(pointer.xy * 0.14, -1.0));
    float3 ray = refract(view, normal, 1.0 / 1.47);
    float bend = length(ray.xy - view.xy) * optics.x;

    float3 lamp = normalize(float3(-0.45 + pointer.x * 0.38, -0.66 + pointer.y * 0.33, 0.74));
    float bias = 0.5 + 0.5 * dot(edgeNormal, normalize(lamp.xy));
    float shine = pow(saturate(dot(normal, lamp)), 9.0);
    float fresnel = 0.035 + 0.965 * pow(saturate(1.0 - normal.z), 4.6);
    float distance = -edge;
    float rim = exp(-abs(distance - (0.9 + bend)) * 0.68);
    float inside = 1.0 - smoothstep(-0.8, 0.5, edge);

    float3 reflected = lerp(float3(0.55, 0.68, 0.95), float3(0.99, 0.96, 0.89), bias);
    reflected = lerp(reflected, 1.0, shine * 0.75);
    float prism = optics.z * bend * bevel;
    float3 bands = exp(-abs(distance - (1.3 + prism * float3(0.45, 1.3, 2.25))) * 0.85);
    float3 color = saturate(reflected + (bands - 0.5) * min(0.32, prism * 0.55));
    float alpha = inside * rim * (0.11 + 0.16 * bias + 0.29 * fresnel + 0.22 * shine);
    alpha *= lerp(1.0, 0.79, optics.w);

    panel[thread.xy] = float4(color.bgr * alpha, alpha);
}

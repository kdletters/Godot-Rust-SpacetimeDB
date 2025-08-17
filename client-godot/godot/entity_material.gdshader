shader_type canvas_item;

uniform vec4 tint : source_color = vec4(1.0);
uniform float edge_softness = 0.0; // 0 为硬边，>0 为柔边
// 以贴图中心为圆心，半径占 UV 的 0.5 即为正好内接圆
void fragment() {
    float d = distance(UV, vec2(0.5));
    float radius = 0.5;
    float alpha = 1.0 - smoothstep(radius - edge_softness, radius, d);
    if (alpha <= 0.0) {
        discard;
    }
    COLOR = tint;
    COLOR.a *= alpha;
}
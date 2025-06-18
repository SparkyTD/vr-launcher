import { Component, createMemo } from 'solid-js';

interface AreaChartProps {
    data: number[];
    color?: string;
    width?: number;
    height?: number;
    min?: number;
    max?: number;
    fillParent?: boolean;
}

const AreaChart: Component<AreaChartProps> = (props) => {
    const color = () => props.color || '#3b82f6';
    const width = () => props.width || 400;
    const height = () => props.height || 200;
    const min = () => props.min ?? 0;
    const max = () => props.max ?? 100;
    const fillParent = () => props.fillParent ?? false;

    const pathData = createMemo(() => {
        const { data } = props;
        if (!data || data.length === 0) return '';

        const w = width();
        const h = height();
        const minVal = min();
        const maxVal = max();
        const range = maxVal - minVal;

        // Calculate points
        const points = data.map((value, index) => {
            const x = (index / (data.length - 1)) * w;
            const y = h - ((value - minVal) / range) * h;
            return { x, y };
        });

        if (points.length < 2) return '';

        // Create smooth curve using cubic bezier curves with proper control points
        let path = `M ${points[0].x} ${points[0].y}`;

        for (let i = 1; i < points.length; i++) {
            const curr = points[i];
            const prev = points[i - 1];
            const next = points[i + 1];
            const prevPrev = points[i - 2];

            // Calculate smooth control points
            const tension = 0.3; // Adjust this to control smoothness (0-1)

            let cp1x, cp1y, cp2x, cp2y;

            // Control point 1 (from previous point)
            if (prevPrev) {
                const dx = curr.x - prevPrev.x;
                const dy = curr.y - prevPrev.y;
                cp1x = prev.x + dx * tension;
                cp1y = prev.y + dy * tension;
            } else {
                cp1x = prev.x + (curr.x - prev.x) * tension;
                cp1y = prev.y + (curr.y - prev.y) * tension;
            }

            // Control point 2 (to current point)
            if (next) {
                const dx = next.x - prev.x;
                const dy = next.y - prev.y;
                cp2x = curr.x - dx * tension;
                cp2y = curr.y - dy * tension;
            } else {
                cp2x = curr.x - (curr.x - prev.x) * tension;
                cp2y = curr.y - (curr.y - prev.y) * tension;
            }

            path += ` C ${cp1x} ${cp1y} ${cp2x} ${cp2y} ${curr.x} ${curr.y}`;
        }

        return path;
    });

    const areaPath = createMemo(() => {
        const linePath = pathData();
        if (!linePath) return '';

        const w = width();
        const h = height();

        // Add area path by going to bottom corners
        return `${linePath} L ${w} ${h} L 0 ${h} Z`;
    });

    const gradientId = `gradient-${Math.random().toString(36).substr(2, 9)}`;

    return (
        <svg
            width={fillParent() ? "100%" : width()}
            height={fillParent() ? "100%" : height()}
            viewBox={`0 0 ${width()} ${height()}`}
            style={fillParent() ? "width: 100%; height: 100%;" : undefined}
            preserveAspectRatio="none"
        >
            <defs>
                <linearGradient id={gradientId} x1="0%" y1="0%" x2="0%" y2="100%">
                    <stop offset="0%" stop-color={color()} stop-opacity="0.3" />
                    <stop offset="100%" stop-color={color()} stop-opacity="0.05" />
                </linearGradient>
            </defs>

            {/* Area fill */}
            <path
                d={areaPath()}
                fill={`url(#${gradientId})`}
                stroke="none"
            />

            {/* Line */}
            <path
                d={pathData()}
                fill="none"
                stroke={color()}
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
            />
        </svg>
    );
};

export default AreaChart;
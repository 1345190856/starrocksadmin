import { Component, Input, Output, EventEmitter, OnInit, OnChanges, SimpleChanges, AfterViewInit, OnDestroy, ElementRef, ChangeDetectorRef, ViewChild, TemplateRef } from '@angular/core';
import { ResourceService, ResourcePanel } from '../../../../@core/services/resource.service';
import { NbThemeService, NbDialogService } from '@nebular/theme';

@Component({
    selector: 'ngx-chart-panel',
    templateUrl: './chart-panel.component.html',
    styleUrls: ['./chart-panel.component.scss']
})
export class ChartPanelComponent implements OnInit, OnChanges, AfterViewInit, OnDestroy {
    @Input() panel: ResourcePanel;
    @Input() start?: Date;
    @Input() end?: Date;
    @Input() dept: string = 'all';
    @Input() colorIndex: number = 0;
    @Input() isShared: boolean = false;

    @Input() country?: string;

    @Output() edit = new EventEmitter<void>();
    @Output() delete = new EventEmitter<void>();
    @Output() toggleWidth = new EventEmitter<void>();
    @Output() visibilityChange = new EventEmitter<boolean>();

    loading = false;
    error: string | null = null;
    isVisible: boolean = true;
    options: any;

    echartsInstance: any;
    legendItems: {
        name: string;
        color: string;
        checked: boolean;
        max?: string;
        min?: string;
        mean?: string;
        last?: string;
        rawMax?: number;
        rawMin?: number;
        rawMean?: number;
        rawLast?: number;
        rawSum?: number;
    }[] = [];

    statValue: string | null = null;
    statLabel: string | null = null;

    tableRows: any[] = [];
    tableCols: string[] = [];

    dataSources: any[] = [];

    theme: any;

    @ViewChild('detailModal') detailModal: TemplateRef<any>;
    private resizeObserver: ResizeObserver;

    constructor(
        private resourceService: ResourceService,
        private themeService: NbThemeService,
        private dialogService: NbDialogService,
        private el: ElementRef,
        private cdr: ChangeDetectorRef
    ) {
        this.themeService.getJsTheme().subscribe(config => {
            this.theme = config;
        });
    }

    ngOnInit() {
        this.resourceService.getDataSources().subscribe(ds => {
            this.dataSources = ds;
            this.fetchData();
        });
    }

    get isSqlSource(): boolean {
        const ds = this.dataSources.find(d => d.id === this.panel.data_source_id);
        return ds?.type === 'mysql' || ds?.type === 'starrocks';
    }

    ngOnChanges(changes: SimpleChanges) {
        if (changes['start'] || changes['end'] || changes['dept'] || changes['panel'] || changes['country']) {
            this.fetchData();
        }
    }

    fetchData() {
        if (!this.panel || !this.start || !this.end) return;
        this.loading = true;
        this.error = null;
        this.options = null;
        this.statValue = null;

        let query = this.panel.promql_query;
        const isSqlSource = this.isSqlSource || /^\s*(SELECT|SHOW|DESCRIBE|EXPLAIN)/i.test(query);
        const deptFilter = this.dept === 'all' ? (isSqlSource ? '%' : '.*') : this.dept;
        query = query.replace(/\$dept/g, deptFilter);

        const durationSec = (this.end.getTime() - this.start.getTime()) / 1000;
        const rangeStr = Math.ceil(durationSec) + 's';

        // SQL friendly variables
        const rangeS = Math.ceil(durationSec);
        const rangeM = Math.ceil(durationSec / 60);
        const rangeH = Math.ceil(durationSec / 3600);

        // Mapping for $source based on country
        const sourceMap: { [key: string]: string } = {
            'China': 'starrocks_cn',
            'Pakistan': 'starrocks_pak',
            'Thailand': 'starrocks_th',
            'Philippines': 'starrocks_ph',
            'Mexico': 'starrocks_mex',
            'Indonesia': 'starrocks_ine'
        };
        const countryKey = this.country || '';
        const sourceName = sourceMap[countryKey] || '';
        query = query.replace(/\$source\b/g, sourceName);

        const formatDate = (date: Date) => {
            const pad = (n: number) => n < 10 ? '0' + n : n;
            return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`;
        };

        query = query.replace(/\$start_time\b/g, formatDate(this.start));
        query = query.replace(/\$end_time\b/g, formatDate(this.end));
        query = query.replace(/\$range_s\b/g, rangeS.toString());
        query = query.replace(/\$range_m\b/g, rangeM.toString());
        query = query.replace(/\$range_h\b/g, rangeH.toString());
        query = query.replace(/\$range\b/g, isSqlSource ? rangeS.toString() : rangeStr);

        // Calculate step based on range
        let step = '15s'; // Default
        if (durationSec > 3600) step = '1m';
        if (durationSec > 3600 * 6) step = '2m';
        if (durationSec > 3600 * 12) step = '5m';
        if (durationSec > 3600 * 24) step = '15m'; // Less Granularity for larger range
        if (durationSec > 3600 * 24 * 7) step = '1h';

        this.resourceService.queryPrometheus(query, this.start, this.end, step, this.panel.data_source_id).subscribe({
            next: (res) => {
                this.processData(res.data);
                this.loading = false;

                // For shared pages, auto-hide if no data
                if (this.isShared) {
                    const hasData = this.options || this.statValue || (this.tableRows && this.tableRows.length > 0);
                    const newVisible = !!hasData;
                    if (this.isVisible !== newVisible) {
                        this.isVisible = newVisible;
                        this.visibilityChange.emit(this.isVisible);
                    }
                }
            },
            error: (err) => {
                console.error(err);
                this.error = 'Query Failed: ' + (err.error?.message || err.message);
                this.loading = false;
                if (this.isShared && this.isVisible) {
                    this.isVisible = false;
                    this.visibilityChange.emit(false);
                }
            }
        });
    }

    processData(data: any) {
        const type = data.resultType; // 'matrix', 'vector', or 'table'
        const result = data.result;

        if (type === 'table') {
            if (this.panel.chart_type === 'pie') {
                this.processTableAsPie(result);
            } else if (this.panel.chart_type === 'line' || this.panel.chart_type === 'bar') {
                this.processTableAsLine(result);
            } else {
                this.processTable(result);
            }
        } else if (this.panel.chart_type === 'stat') {
            this.processStat(result, type);
        } else {
            this.processChart(result, type);
        }
    }

    processTableAsLine(result: any[]) {
        if (!result || result.length === 0) {
            this.options = null;
            return;
        }

        const firstRow = result[0];
        const keys = Object.keys(firstRow);

        let timeKey = '';
        let seriesKey = '';
        let valueKey = '';

        // Heuristic to find keys
        // X-Axis (Time): look for keys containing 'time', 'date', 'day' or values matching date pattern
        for (const key of keys) {
            const lowKey = key.toLowerCase();
            const val = firstRow[key];
            if (lowKey.includes('time') || lowKey.includes('date') || lowKey.includes('day')) {
                timeKey = key;
                break;
            }
            if (typeof val === 'string' && /^\d{4}[-/]\d{2}[-/]\d{2}/.test(val)) {
                timeKey = key;
                break;
            }
        }

        // Value: look for numeric keys or values
        for (const key of keys) {
            if (key === timeKey) continue;
            const lowKey = key.toLowerCase();
            const val = firstRow[key];
            if (lowKey.includes('count') || lowKey.includes('total') || lowKey.includes('value') || lowKey.includes('sum')) {
                valueKey = key;
                break;
            }
            if (typeof val === 'number') {
                valueKey = key;
                break;
            }
        }

        // Series: first string key that isn't timeKey
        for (const key of keys) {
            if (key === timeKey || key === valueKey) continue;
            const val = firstRow[key];
            if (typeof val === 'string') {
                seriesKey = key;
                break;
            }
        }

        // Fallbacks if heuristic failed
        if (!timeKey) timeKey = keys[0];
        if (!valueKey) valueKey = keys.find(k => k !== timeKey) || keys[0];
        if (!seriesKey) seriesKey = keys.find(k => k !== timeKey && k !== valueKey) || '';

        // Group and aggregate data by seriesKey and timestamp
        const allTimestampsSet = new Set<number>();
        const seriesDataMap = new Map<string, Map<number, number>>();

        result.forEach(row => {
            const seriesName = seriesKey ? String(row[seriesKey] || 'Total') : 'Total';
            if (!seriesDataMap.has(seriesName)) seriesDataMap.set(seriesName, new Map());

            let timeVal = row[timeKey];
            let timestamp: number;
            if (typeof timeVal === 'number') {
                timestamp = timeVal > 10000000000 ? timeVal : timeVal * 1000;
            } else {
                // Normalize to start of day if it's a date string to ensure alignment
                const d = new Date(timeVal);
                d.setHours(0, 0, 0, 0);
                timestamp = d.getTime();
            }
            allTimestampsSet.add(timestamp);

            const val = typeof row[valueKey] === 'number' ? row[valueKey] : parseFloat(row[valueKey]) || 0;
            const dataMap = seriesDataMap.get(seriesName);
            dataMap.set(timestamp, (dataMap.get(timestamp) || 0) + val);
        });

        // Collect all timestamps across all series and sort them
        const allTimestamps = Array.from(allTimestampsSet).sort((a, b) => a - b);

        // Dynamic sorting: Biggest total at the bottom (index 0)
        const seriesTotals = new Map<string, number>();
        seriesDataMap.forEach((dataMap, name) => {
            let total = 0;
            dataMap.forEach(v => total += v);
            seriesTotals.set(name, total);
        });

        const seriesNames = Array.from(seriesDataMap.keys()).sort((a, b) => {
            return (seriesTotals.get(b) || 0) - (seriesTotals.get(a) || 0);
        });

        const seriesData: any[] = [];
        const legendData: string[] = [];
        const legendStats: any[] = [];

        const defaultColors = [
            '#5470c6', '#91cc75', '#fac858', '#ee6666', '#73c0de', '#3ba272', '#fc8452', '#9a60b4', '#ea7ccc'
        ];
        const startIndex = (this.colorIndex || 0) % defaultColors.length;
        const colors = [...defaultColors.slice(startIndex), ...defaultColors.slice(0, startIndex)];

        seriesNames.forEach((name, idx) => {
            const dataMap = seriesDataMap.get(name);
            // Pad data for all timestamps to ensure perfectly aligned stacks
            const data = allTimestamps.map(ts => [ts, dataMap.get(ts) || 0]);

            legendData.push(name);

            const values = data.map(d => d[1]);
            let max = -Infinity, min = Infinity, sum = 0;
            if (values.length > 0) {
                for (const v of values) {
                    if (v > max) max = v;
                    if (v < min) min = v;
                    sum += v;
                }
            } else {
                max = 0; min = 0;
            }
            const mean = values.length > 0 ? (sum / values.length) : 0;
            const last = values.length > 0 ? values[values.length - 1] : 0;
            legendStats.push({ max, min, mean, last, sum });

            seriesData.push({
                name: name,
                type: this.panel.chart_type,
                data: data,
                smooth: true,
                stack: this.panel.chart_type === 'bar' ? '总量' : undefined,
                label: this.panel.chart_type === 'bar' ? {
                    show: true,
                    position: 'inside',
                    formatter: (params: any) => params.value[1] > 0 ? params.value[1].toFixed(0) : ''
                } : undefined,
                showSymbol: data.length < 50,
                tooltip: {
                    valueFormatter: (value) => this.formatValue(value as number)
                }
            });
        });

        const duration = this.end.getTime() - this.start.getTime();
        let timeFormat = '{MM}-{dd}';
        if (duration <= 24 * 3600 * 1000 + 5000) timeFormat = '{HH}:{mm}';

        this.legendItems = legendData.map((name, index) => ({
            name,
            color: colors[index % colors.length],
            checked: true,
            max: this.formatValue(legendStats[index].max),
            min: this.formatValue(legendStats[index].min),
            mean: this.formatValue(legendStats[index].mean),
            last: this.formatValue(legendStats[index].last),
            rawMax: legendStats[index].max,
            rawMin: legendStats[index].min,
            rawMean: legendStats[index].mean,
            rawLast: legendStats[index].last,
            rawSum: legendStats[index].sum
        }));

        this.options = {
            tooltip: {
                trigger: 'axis',
                appendToBody: true
            },
            legend: {
                show: false
            },
            grid: {
                top: '5%',
                left: '3%',
                right: '4%',
                bottom: '3%',
                containLabel: true
            },
            xAxis: {
                type: 'time',
                axisLabel: { formatter: timeFormat },
                splitLine: { show: false }
            },
            yAxis: {
                type: 'value',
                axisLabel: { formatter: (value) => this.formatValue(value) }
            },
            series: seriesData,
            color: colors
        };

        this.cdr.markForCheck();
    }
    processTableAsPie(result: any[]) {
        if (!result || result.length === 0) {
            this.options = null;
            return;
        }

        const firstRow = result[0];
        const keys = Object.keys(firstRow);

        let labelKey = '';
        let valueKey = '';

        for (const key of keys) {
            const val = firstRow[key];
            const isNumericValue = typeof val === 'number';
            const isNamedLikeValue = /count|sum|value|total|amount/i.test(key);
            const isConvertibleToNumber = val !== null && val !== '' && !isNaN(Number(val));

            if (isNumericValue || (isNamedLikeValue && isConvertibleToNumber)) {
                valueKey = key;
                break;
            }
        }

        for (const key of keys) {
            if (key === valueKey) continue;
            const val = firstRow[key];
            if (typeof val === 'string' && isNaN(Number(val))) {
                labelKey = key;
                break;
            }
        }

        if (!valueKey) {
            for (const key of keys) {
                if (key === labelKey) continue;
                if (!isNaN(Number(firstRow[key]))) {
                    valueKey = key;
                    break;
                }
            }
        }
        if (!labelKey) {
            for (const key of keys) {
                if (key === valueKey) continue;
                labelKey = key;
                break;
            }
        }

        if (!labelKey || !valueKey) {
            console.warn('[ResourceChart] Failed to find Pie Chart keys in SQL result:', {
                labelKey, valueKey, keys, firstRow
            });
            this.processTable(result);
            return;
        }

        const aggregated: Map<string, number> = new Map();
        result.forEach(row => {
            const d = this.findDeptValue(row);

            if (this.dept !== 'all') {
                if (!d || d !== this.dept) return;
            }

            const label = String(row[labelKey] || 'Other');
            if (this.dept !== 'all' && (label === 'Other' || label === '未知')) return;

            const val = row[valueKey];
            const num = typeof val === 'number' ? val : (parseFloat(val) || 0);
            aggregated.set(label, (aggregated.get(label) || 0) + num);
        });

        const seriesData = Array.from(aggregated.entries()).map(([name, value]) => ({
            name,
            value
        }));

        this.renderPie(seriesData);
        this.cdr.markForCheck();
    }

    processTable(result: any[]) {
        if (!result || result.length === 0) {
            this.tableRows = [];
            this.tableCols = [];
            return;
        }

        const originalCols = Object.keys(result[0]);
        this.tableCols = originalCols;

        if (!this.dept || this.dept === 'all') {
            this.tableRows = result;
            return;
        }

        // Identify Department columns (Group Anchors)
        const groupAnchors: number[] = [];
        for (let i = 0; i < originalCols.length; i++) {
            const k = originalCols[i];
            const isDept = k.toLowerCase().includes('department') || k.includes('部门') || k.toLowerCase() === 'name' || k === '名称';
            if (isDept) {
                groupAnchors.push(i);
            }
        }

        // FIX: If we only have one department column, it's a standard aggregated table.
        // We should just filter rows and NOT discard columns after index 3.
        if (groupAnchors.length <= 1) {
            this.tableRows = result.filter(row => {
                if (groupAnchors.length === 0) return true;
                return groupAnchors.some(idx => row[originalCols[idx]] === this.dept);
            });
            return;
        }

        // --- Multi-Anchor Compaction Logic (For Top 10 CPU/Memory panels) ---
        const filteredGroups: any[][] = [];
        for (const anchorIdx of groupAnchors) {
            const groupData = result.filter(row => row[originalCols[anchorIdx]] === this.dept)
                .map(row => {
                    const obj: any = {};
                    for (let offset = 0; offset < 3; offset++) {
                        const idx = anchorIdx + offset;
                        if (idx < originalCols.length) {
                            obj[originalCols[idx]] = row[originalCols[idx]];
                        }
                    }
                    return obj;
                });
            filteredGroups.push(groupData);
        }

        const maxLen = Math.max(...filteredGroups.map(g => g.length), 0);
        const compactedRows: any[] = [];

        for (let i = 0; i < maxLen; i++) {
            const row: any = {};
            const rankKey = originalCols.find(k => k === '排名' || k.toLowerCase() === 'rank');
            if (rankKey) {
                row[rankKey] = i + 1;
            }

            for (let gIdx = 0; gIdx < groupAnchors.length; gIdx++) {
                const item = filteredGroups[gIdx][i];
                if (item) {
                    Object.assign(row, item);
                } else {
                    const anchorIdx = groupAnchors[gIdx];
                    for (let offset = 0; offset < 3; offset++) {
                        const idx = anchorIdx + offset;
                        if (idx < originalCols.length) {
                            row[originalCols[idx]] = '';
                        }
                    }
                }
            }
            compactedRows.push(row);
        }

        this.tableRows = compactedRows;
    }

    shouldHighlight(row: any, col: string): boolean {
        return false;
    }

    private findDeptValue(row: any): string | null {
        if (!row) return null;
        if (row.metric) {
            const m = row.metric;
            return m['department'] || m['部门'] || m['name'] || m['名称'] || null;
        }
        const priority = ['department', '部门', 'name', '名称'];
        for (const k of priority) {
            if (row[k]) return row[k];
        }
        const keys = Object.keys(row);
        for (const k of keys) {
            if (k.toLowerCase().includes('department') || k.includes('部门')) {
                if (this.dept !== 'all' && row[k] === this.dept) {
                    return row[k];
                }
            }
        }
        for (const k of keys) {
            if (k.toLowerCase().includes('department') || k.includes('部门')) {
                return row[k];
            }
        }
        return null;
    }

    processStat(result: any[], type: string) {
        let filteredResult = result;
        if (this.dept !== 'all') {
            filteredResult = result.filter(r => {
                const d = r.metric['department'] || r.metric['部门'] || r.metric['name'] || r.metric['名称'];
                if (d === 'Other' || d === '未知') return false;
                return d === this.dept;
            });
        }

        let val = 0;
        if (type === 'vector' && filteredResult.length > 0) {
            val = filteredResult.reduce((acc, r) => acc + parseFloat(r.value[1]), 0);
        } else if (type === 'matrix' && filteredResult.length > 0) {
            val = filteredResult.reduce((acc, r) => {
                const values = r.values;
                return acc + parseFloat(values[values.length - 1][1]);
            }, 0);
        }

        this.statValue = this.formatValue(val);
        this.statLabel = '';
    }

    processChart(result: any[], type: string) {
        if (type !== 'matrix') {
            if (this.panel.chart_type === 'pie') {
            } else {
                this.error = 'Chart requires range query (matrix)';
                return;
            }
        }

        const defaultColors = [
            '#5470c6', '#91cc75', '#fac858', '#ee6666', '#73c0de', '#3ba272', '#fc8452', '#9a60b4', '#ea7ccc',
            '#c23531', '#2f4554', '#61a0a8', '#d48265', '#91c7ae', '#749f83', '#ca8622', '#bda29a', '#6e7074'
        ];

        const startIndex = (this.colorIndex || 0) % defaultColors.length;
        const colors = [...defaultColors.slice(startIndex), ...defaultColors.slice(0, startIndex)];

        if (this.panel.chart_type === 'line' || this.panel.chart_type === 'bar') {
            const series = [];
            const legendData = [];
            const legendStats: any[] = [];

            result.forEach((r: any) => {
                if (this.dept !== 'all') {
                    const metricDept = r.metric['department'] || r.metric['部门'] || r.metric['name'] || r.metric['名称'];
                    if (metricDept !== this.dept) return;
                }

                let name = this.formatLegend(r.metric);
                legendData.push(name);

                const data = r.values.map((v: any) => [v[0] * 1000, parseFloat(v[1])]);

                const values = data.map((d: any) => d[1]);
                let max = -Infinity, min = Infinity, sum = 0;
                if (values.length > 0) {
                    for (const v of values) {
                        if (v > max) max = v;
                        if (v < min) min = v;
                        sum += v;
                    }
                } else {
                    max = 0; min = 0;
                }
                const mean = values.length > 0 ? (sum / values.length) : 0;
                const last = values.length > 0 ? values[values.length - 1] : 0;
                legendStats.push({ max, min, mean, last, sum });

                series.push({
                    name: name,
                    type: this.panel.chart_type,
                    data: data,
                    smooth: true,
                    stack: this.panel.chart_type === 'bar' ? '总量' : undefined,
                    label: this.panel.chart_type === 'bar' ? {
                        show: true,
                        position: 'inside',
                        formatter: (params: any) => params.value[1] > 0 ? params.value[1].toFixed(0) : ''
                    } : undefined,
                    showSymbol: false,
                    tooltip: {
                        valueFormatter: (value) => this.formatValue(value as number)
                    }
                });
            });

            const duration = this.end.getTime() - this.start.getTime();
            let minInterval = 0;
            let timeFormat = '{value:%H:%M}';

            if (duration <= 3600 * 1000 + 5000) {
                minInterval = 5 * 60 * 1000;
                timeFormat = '{HH}:{mm}';
            } else if (duration <= 6 * 3600 * 1000 + 5000) {
                minInterval = 30 * 60 * 1000;
                timeFormat = '{HH}:{mm}';
            } else if (duration <= 12 * 3600 * 1000 + 5000) {
                minInterval = 60 * 60 * 1000;
                timeFormat = '{HH}:{mm}';
            } else if (duration <= 24 * 3600 * 1000 + 5000) {
                minInterval = 2 * 3600 * 1000;
                timeFormat = '{HH}:{mm}';
            } else {
                minInterval = 24 * 3600 * 1000;
                timeFormat = '{MM}-{dd}';
            }

            this.legendItems = legendData.map((name, index) => ({
                name,
                color: colors[index % colors.length],
                checked: true,
                max: this.formatValue(legendStats[index].max),
                min: this.formatValue(legendStats[index].min),
                mean: this.formatValue(legendStats[index].mean),
                last: this.formatValue(legendStats[index].last),
                rawMax: legendStats[index].max,
                rawMin: legendStats[index].min,
                rawMean: legendStats[index].mean,
                rawLast: legendStats[index].last,
                rawSum: legendStats[index].sum
            }));

            this.options = {
                tooltip: {
                    trigger: 'axis',
                    appendToBody: true,
                    confine: false,
                    className: 'echarts-tooltip'
                },
                legend: {
                    show: false,
                    type: 'scroll'
                },
                grid: {
                    top: '5%',
                    left: '3%',
                    right: '4%',
                    bottom: '3%',
                    containLabel: true
                },
                xAxis: {
                    type: 'time',
                    boundaryGap: false,
                    min: this.start.getTime(),
                    max: this.end.getTime(),
                    minInterval: minInterval,
                    axisLabel: {
                        formatter: timeFormat
                    },
                    splitLine: { show: false }
                },
                yAxis: {
                    type: 'value',
                    scale: true,
                    axisLabel: {
                        formatter: (value) => this.formatValue(value)
                    }
                },
                series: series,
                color: colors
            };
        }
        else if (this.panel.chart_type === 'pie') {
            const seriesData = [];
            result.forEach((r: any) => {
                if (this.dept !== 'all') {
                    const metricDept = r.metric['department'] || r.metric['部门'] || r.metric['name'] || r.metric['名称'];
                    if (metricDept && metricDept !== this.dept) return;
                }

                let val = 0;
                if (type === 'vector') val = parseFloat(r.value[1]);
                else val = parseFloat(r.values[r.values.length - 1][1]);
                seriesData.push({ value: val, name: this.formatLegend(r.metric) });
            });

            this.renderPie(seriesData);
        }
    }

    renderPie(seriesData: any[]) {
        const defaultColors = [
            '#5470c6', '#91cc75', '#fac858', '#ee6666', '#73c0de', '#3ba272', '#fc8452', '#9a60b4', '#ea7ccc',
            '#c23531', '#2f4554', '#61a0a8', '#d48265', '#91c7ae', '#749f83', '#ca8622', '#bda29a', '#6e7074'
        ];
        const startIndex = (this.colorIndex || 0) % defaultColors.length;
        const colors = [...defaultColors.slice(startIndex), ...defaultColors.slice(0, startIndex)];

        this.legendItems = seriesData.map((d, index) => ({
            name: d.name,
            color: colors[index % colors.length],
            checked: true,
            rawLast: d.value,
            rawMax: d.value,
            rawMin: d.value,
            rawMean: d.value,
            rawSum: d.value,
            last: this.formatValue(d.value),
            max: this.formatValue(d.value),
            min: this.formatValue(d.value),
            mean: this.formatValue(d.value)
        }));

        if (this.isShared) {
            const total = seriesData.reduce((acc, d) => acc + (d.value || 0), 0);
            if (total === 0) {
                if (this.isVisible) {
                    this.isVisible = false;
                    this.visibilityChange.emit(false);
                }
                this.options = null;
                return;
            }
        }

        this.options = {
            tooltip: {
                trigger: 'item',
                formatter: (params: any) => {
                    return `${params.marker} ${params.name}: <b>${this.formatValue(params.value)}</b> (${params.percent}%)`;
                }
            },
            legend: {
                show: false,
                type: 'scroll'
            },
            series: [{
                type: 'pie',
                radius: ['40%', '70%'],
                avoidLabelOverlap: false,
                itemStyle: { borderRadius: 10, borderColor: '#fff', borderWidth: 2 },
                label: {
                    show: true,
                    position: 'outside',
                    formatter: (params: any) => {
                        return `${params.name}: ${this.formatValue(params.value)}`;
                    }
                },
                data: seriesData
            }],
            color: colors
        };
    }

    formatValue(value: number): string {
        if (value === null || value === undefined) return '-';
        const unit = this.panel.config?.unit;

        if (!unit) return value.toFixed(2);

        if (unit.toLowerCase() === 'bytes' || unit.toLowerCase() === 'b') {
            if (value === 0) return '0 B';
            const k = 1024;
            const sizes = ['B', 'KB', 'MB', 'GB', 'TB', 'PB'];
            const i = Math.floor(Math.log(Math.abs(value)) / Math.log(k));
            if (i < 0) return value + ' B';
            return parseFloat((value / Math.pow(k, i)).toFixed(2)) + ' ' + (sizes[i] || 'B');
        }

        if (unit === '%') {
            return value.toFixed(2) + '%';
        }

        return value.toFixed(2) + ' ' + unit;
    }

    formatLegend(metric: any): string {
        if (this.panel.config?.legendFormat) {
            let fmt = this.panel.config.legendFormat;
            for (const key in metric) {
                const re = new RegExp(`\\{\\{${key}\\}\\}`, 'g');
                fmt = fmt.replace(re, metric[key]);
            }
            return fmt;
        }

        const keys = Object.keys(metric).filter(k => k !== '__name__');
        if (keys.length === 0) return 'Value';
        return keys.map(k => metric[k]).join('-');
    }
    onChartInit(ec: any) {
        this.echartsInstance = ec;
    }

    toggleLegend(item: any) {
        const checkedCount = this.legendItems.filter(i => i.checked).length;
        const isCurrentlyOnlyChecked = checkedCount === 1 && item.checked;

        if (isCurrentlyOnlyChecked) {
            this.legendItems.forEach(i => i.checked = true);
        } else {
            this.legendItems.forEach(i => i.checked = (i === item));
        }

        if (this.echartsInstance) {
            const selectedMap: any = {};
            this.legendItems.forEach(i => selectedMap[i.name] = i.checked);
            this.echartsInstance.setOption({
                legend: { selected: selectedMap }
            });
        }
    }

    isLongText(val: any): boolean {
        if (val === null || val === undefined) return false;
        const s = String(val);
        return s.length > 20 || s.includes('\n');
    }

    shouldShowViewIcon(col: string, val: any): boolean {
        if (!this.isLongText(val)) return false;
        const lowerCol = col.toLowerCase();
        return lowerCol.includes('sql');
    }

    showDetail(title: string, content: string) {
        this.dialogService.open(this.detailModal, {
            context: { title, content },
            autoFocus: false,
            closeOnBackdropClick: true
        });
    }

    ngAfterViewInit() {
        if (typeof ResizeObserver !== 'undefined') {
            this.resizeObserver = new ResizeObserver(() => {
                if (this.echartsInstance) {
                    this.echartsInstance.resize();
                }
            });
            this.resizeObserver.observe(this.el.nativeElement);
        }
    }

    ngOnDestroy() {
        if (this.resizeObserver) {
            this.resizeObserver.disconnect();
        }
    }
}

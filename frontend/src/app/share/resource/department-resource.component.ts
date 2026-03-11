
import { Component, OnInit, ViewChildren, QueryList } from '@angular/core';
import { NbToastrService } from '@nebular/theme';
import { ResourceService, ResourcePanel, ResourceDataSource } from '../../@core/services/resource.service';
import { forkJoin } from 'rxjs';
import { ChartPanelComponent } from '../../pages/resource/components/chart-panel/chart-panel.component';
import html2canvas from 'html2canvas';
import jsPDF from 'jspdf';

interface CountryGroup {
    country: string;
    panels: ResourcePanel[];
}

@Component({
    selector: 'ngx-department-resource',
    templateUrl: './department-resource.component.html',
    styleUrls: ['./department-resource.component.scss']
})
export class DepartmentResourceComponent implements OnInit {

    departments = [
        '组件研发组', '智能模型中心', '用户增长组', '研发组',
        '信贷平台部', '信贷策略中心', '唯渡', '数据开发组', '数据产品组',
        '商业分析组', '前端组', '决策平台部', '经营分析组', '后端组',
        '国内机构运营组', '国际业务组', '贷后业务组', '贷后策略部',
        '创新策略组', 'OA组'
    ];
    selectedDept: string = 'all';
    timeRange: string = '1h';

    countries = ['China', 'Pakistan', 'Thailand', 'Philippines', 'Mexico', 'Indonesia', 'Common'];
    selectedCountry: string = 'China'; // Default

    customStart: Date;
    customEnd: Date;
    startTimeInput: string;
    endTimeInput: string;

    panels: ResourcePanel[] = [];
    dataSources: ResourceDataSource[] = [];

    // Group panels by country for display
    displayedGroups: CountryGroup[] = [];

    summary: string = '';
    exporting = false;

    @ViewChildren(ChartPanelComponent) chartPanels: QueryList<ChartPanelComponent>;

    constructor(
        private resourceService: ResourceService, // Will be SharedResourceService
        private toastrService: NbToastrService
    ) { }

    ngOnInit() {
        this.setQuickRange('1h');
        this.loadPanels();
    }

    loadPanels() {
        forkJoin({
            panels: this.resourceService.getPanels(),
            dataSources: this.resourceService.getDataSources()
        }).subscribe(res => {
            this.panels = res.panels;
            this.dataSources = res.dataSources;
            this.applyFilters();
        });
    }

    applyFilters() {
        // 1. Filter by section (department only)
        const deptPanels = this.panels.filter(p => p.section === 'department');

        // 2. Group by country based on selection
        this.displayedGroups = [];

        const targetCountries = (this.selectedCountry === 'All' || this.selectedCountry === '所有国家')
            ? this.countries
            : [this.selectedCountry];

        targetCountries.forEach(country => {
            // Find panels belonging to this country
            const countryPanels = deptPanels.filter(p => {
                const region = p.country || this.dataSources.find(d => d.id === p.data_source_id)?.region || 'China';
                return region === country;
            }).sort((a, b) => (a.display_order || 0) - (b.display_order || 0));

            if (countryPanels.length > 0) {
                this.displayedGroups.push({
                    country: country,
                    panels: countryPanels
                });
            }
        });
    }

    onFilterChange() {
        if (this.timeRange !== 'custom') {
            this.setQuickRange(this.timeRange);
        }
        // Re-apply filters if country changed (bound to ngModel)
        this.applyFilters();
    }

    setQuickRange(range: string) {
        this.timeRange = range;
        const end = new Date();
        const start = new Date();

        switch (range) {
            case '1h': start.setTime(end.getTime() - 3600 * 1000); break;
            case '6h': start.setTime(end.getTime() - 6 * 3600 * 1000); break;
            case '12h': start.setTime(end.getTime() - 12 * 3600 * 1000); break;
            case '24h': start.setTime(end.getTime() - 24 * 3600 * 1000); break;
            case '3d': start.setTime(end.getTime() - 3 * 24 * 3600 * 1000); break;
            case '7d': start.setTime(end.getTime() - 7 * 24 * 3600 * 1000); break;
        }

        this.customStart = start;
        this.customEnd = end;
        this.startTimeInput = this.formatDate(start);
        this.endTimeInput = this.formatDate(end);
    }

    onDateInputChange() {
        this.timeRange = 'custom';
        if (this.startTimeInput) this.customStart = new Date(this.startTimeInput);
        if (this.endTimeInput) this.customEnd = new Date(this.endTimeInput);
    }

    refreshData() {
        this.customStart = new Date(this.customStart);
        this.customEnd = new Date(this.customEnd);
        this.applyFilters();
    }

    private formatDate(date: Date): string {
        const pad = (n: number) => n < 10 ? '0' + n : n;
        return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}T${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`;
    }

    analyze(event?: Event) {
        if (event) event.stopPropagation();
        this.toastrService.info(`正在分析 [${this.selectedDept}] 资源使用情况...`, '智能分析');
        this.analyzeDepartmentLocally();
    }

    private analyzeDepartmentLocally() {
        const isAllDepts = this.selectedDept === 'all';
        let fullSummary = isAllDepts ? '各部门资源概览 (Top 3):\n' : `部门 [${this.selectedDept}] 资源概览:\n`;

        // Helper to match panels
        // We need to match panels to displayedGroups. 
        // ChartPanelComponent is a flat list. We can match by panel ID.
        const panelsComponents = this.chartPanels.toArray();

        this.displayedGroups.forEach(group => {
            const country = group.country;
            const countryHeader = this.selectedCountry === 'China' ? '' : `\n=== ${country} ===\n`;
            if (this.displayedGroups.length > 1) {
                fullSummary += countryHeader;
            }

            // Find relevant chart components for this group
            // Filter components where component.panel.id is in group.panels
            const groupPanelIds = new Set(group.panels.map(p => p.id));
            const groupComponents = panelsComponents.filter(cp => groupPanelIds.has(cp.panel.id));

            const findPanel = (part: string) => {
                const p = part.toLowerCase();
                return groupComponents.find(cp =>
                    (cp.panel.title && cp.panel.title.toLowerCase().includes(p)) ||
                    (cp.panel.promql_query && cp.panel.promql_query.toLowerCase().includes(p))
                );
            };

            const cpuPanel = findPanel('avg_cpu_cores');
            const memPanel = findPanel('avg_mem_bytes');

            const perfPanel = findPanel('starrocks_audit_tbl__');
            const abnormalCpuPanel = findPanel('cpu开销');
            const abnormalMemPanel = findPanel('内存开销');
            const abnormalLatPanel = findPanel('执行时间查询');
            const abnormalRowPanel = findPanel('扫描行数查询');

            const getStat = (cp: ChartPanelComponent | undefined, unit: string, factor: number = 1) => {
                if (!cp || !cp.legendItems || cp.legendItems.length === 0) return '无数据';
                if (!isAllDepts) {
                    const item = cp.legendItems.find(i => i.name === this.selectedDept);
                    return item ? `峰值: ${((item.rawMax || 0) * factor).toFixed(2)}${unit}, 均值: ${((item.rawMean || 0) * factor).toFixed(2)}${unit}` : '无数据';
                }
                // For 'all', show top 3 by peak
                return cp.legendItems
                    .filter(item => item.name !== '未知')
                    .sort((a, b) => (b.rawMax || 0) - (a.rawMax || 0))
                    .slice(0, 3)
                    .map(item => `${item.name}(峰${((item.rawMax || 0) * factor).toFixed(1)}${unit})`)
                    .join('、');
            };

            const cpuStats = getStat(cpuPanel, '核');
            const memStats = getStat(memPanel, 'G', 1.0 / (1024 * 1024 * 1024));

            // Latency Analysis
            let latStats = '无数据';
            if (perfPanel && perfPanel.tableRows && perfPanel.tableRows.length > 0) {
                const getVal = (r: any, ...keys: string[]) => {
                    for (const k of keys) if (r[k] !== undefined) return r[k];
                    return 0;
                };
                const filteredRows = perfPanel.tableRows.filter(r => (r['部门'] || r['department'] || '').toString() !== '未知');

                if (!isAllDepts) {
                    const row = filteredRows.find(r => (r['部门'] || r['department']) === this.selectedDept);
                    if (row) {
                        const peak = getVal(row, '时延_峰值', 'lat_peak');
                        const p99 = getVal(row, '时延_P99', 'lat_p99');
                        latStats = `峰值: ${peak}s, P99: ${p99}s`;
                    }
                } else {
                    // Top 3 by peak
                    latStats = filteredRows
                        .sort((a, b) => getVal(b, '时延_峰值', 'lat_peak') - getVal(a, '时延_峰值', 'lat_peak'))
                        .slice(0, 3)
                        .map(r => `${r['部门'] || r['department']}(峰${getVal(r, '时延_峰值', 'lat_peak')}s)`)
                        .join('、');
                }
            }

            // Exception Analysis
            const getExceptionCountStr = (cp: ChartPanelComponent | undefined) => {
                if (!cp || !cp.legendItems || cp.legendItems.length === 0) return '0';
                if (!isAllDepts) {
                    const item = cp.legendItems.find(i => i.name === this.selectedDept);
                    return item ? (item.rawSum || 0).toFixed(0) : '0';
                }
                return cp.legendItems
                    .filter(item => item.name !== '未知')
                    .sort((a, b) => (b.rawSum || 0) - (a.rawSum || 0))
                    .slice(0, 3)
                    .map(item => `${item.name}(${(item.rawSum || 0).toFixed(0)}个)`)
                    .join('、');
            };

            const excCpu = getExceptionCountStr(abnormalCpuPanel);
            const excMem = getExceptionCountStr(abnormalMemPanel);
            const excLat = getExceptionCountStr(abnormalLatPanel);
            const excRow = getExceptionCountStr(abnormalRowPanel);

            // Aggregate all exceptions for "SQL Blacklist Top"
            const exceptionPanels = [abnormalCpuPanel, abnormalMemPanel, abnormalLatPanel, abnormalRowPanel];
            const deptCounts: { [name: string]: number } = {};
            exceptionPanels.forEach(cp => {
                if (!cp || !cp.legendItems) return;
                cp.legendItems.forEach(item => {
                    if (item.name === '未知') return;
                    deptCounts[item.name] = (deptCounts[item.name] || 0) + (item.rawSum || 0);
                });
            });

            const blacklistTop = Object.entries(deptCounts)
                .sort((a, b) => b[1] - a[1])
                .slice(0, 5)
                .map(([name, count]) => `${name}（${count.toFixed(0)}个）`)
                .join('、');

            const blacklistStr = blacklistTop ? `sql黑榜top：${blacklistTop}` : 'sql黑榜top：暂无数据';

            fullSummary += `- CPU使用: ${cpuStats}\n` +
                `- 内存使用: ${memStats}\n` +
                `- 时延: ${latStats}\n` +
                `异常查询统计:\n` +
                ` - CPU开销: ${excCpu}\n` +
                ` - 内存开销: ${excMem}\n` +
                ` - 执行时间: ${excLat}\n` +
                ` - 扫描行数: ${excRow}\n` +
                `${blacklistStr}\n`;
        });

        fullSummary += `\n该分析基于所选时间范围内的数据统计。`;
        this.summary = fullSummary;
    }

    async exportPDF() {
        this.exporting = true;
        const container = document.getElementById('dashboard-content');
        if (!container) {
            this.exporting = false;
            return;
        }

        try {
            container.classList.add('exporting');
            await new Promise(resolve => setTimeout(resolve, 300)); // Wait for render

            const pdf = new jsPDF({
                orientation: 'p',
                unit: 'mm',
                format: 'a4',
                compress: true
            });

            const pdfWidth = pdf.internal.pageSize.getWidth();
            const pageHeight = pdf.internal.pageSize.getHeight();
            const margin = 5;
            let currentY = margin;

            // Capture the whole container or section by section?
            // Since it's just Department resource, maybe just capture the accordion body content if expanded
            // Structure: nb-card-body -> nb-accordion -> nb-accordion-item ...
            // Let's capture the main container inside accordion body to avoid scrolling issues if possible
            // But 'dashboard-content' is simpler.

            // Better approach: Capture each group (country) separately to handle pagination.
            const summaryEl = container.querySelector('.summary-export-view');
            const groups = Array.from(container.querySelectorAll('.country-group-container'));

            const targets: Element[] = [];
            if (summaryEl && this.summary) {
                targets.push(summaryEl);
            }

            if (groups.length > 0) {
                targets.push(...groups);
            } else {
                // If no groups, we might still want the container but avoid double summary?
                // For simplicity, if groups are missing, just use container as before.
                if (targets.length === 0) targets.push(container);
            }

            const capturedImages: { img: string; width: number; height: number }[] = [];

            for (let i = 0; i < targets.length; i++) {
                const element = targets[i] as HTMLElement;

                // Yield to browser to prevent "Page Unresponsive"
                await new Promise(resolve => setTimeout(resolve, 50));

                const originalStyle = element.style.display;
                if (window.getComputedStyle(element).display === 'none') {
                    element.style.setProperty('display', 'block', 'important');
                }

                try {
                    const canvas = await html2canvas(element, {
                        scale: 1, // Standard resolution for speed
                        useCORS: true,
                        logging: false,
                        backgroundColor: '#ffffff',
                        ignoreElements: (el) => el.classList.contains('no-export')
                    });

                    element.style.display = originalStyle;
                    const imgData = canvas.toDataURL('image/jpeg', 0.8);

                    if (imgData && imgData !== 'data:,') {
                        capturedImages.push({
                            img: imgData,
                            width: canvas.width,
                            height: canvas.height
                        });
                    }

                    // Free memory
                    canvas.width = 0;
                    canvas.height = 0;
                } catch (err) {
                    console.error('Capture failed for element', i, err);
                }
            }

            for (let i = 0; i < capturedImages.length; i++) {
                const { img, width, height } = capturedImages[i];
                if (!img || img === 'data:,') continue;

                const imgProps = pdf.getImageProperties(img);
                if (imgProps.fileType === 'UNKNOWN') continue;

                const imgWidth = pdfWidth - 2 * margin;
                const imgHeight = (height * imgWidth) / width;

                if (currentY + imgHeight > pageHeight - margin && i > 0) {
                    pdf.addPage();
                    currentY = margin;
                }

                pdf.addImage(img, 'JPEG', margin, currentY, imgWidth, imgHeight, undefined, 'FAST');
                currentY += imgHeight + 5;
            }

            pdf.save(`department-resource-${this.selectedDept}-${new Date().getTime()}.pdf`);
            this.toastrService.success('Export complete', 'PDF Export');

        } catch (e: any) {
            console.error(e);
            this.toastrService.danger(`Export failed: ${e.message}`, 'Error');
        } finally {
            container.classList.remove('exporting');
            this.exporting = false;
        }
    }
}

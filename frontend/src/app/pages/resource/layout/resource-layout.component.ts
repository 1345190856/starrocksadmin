
import { Component, OnInit, ViewChildren, QueryList } from '@angular/core';
import { NbDialogService, NbToastrService } from '@nebular/theme';
import { ResourceService, ResourcePanel, ResourceDataSource } from '../../../@core/services/resource.service';
import { forkJoin } from 'rxjs';
import { PanelEditorComponent } from '../components/panel-editor/panel-editor.component';
import { ChartPanelComponent } from '../components/chart-panel/chart-panel.component';
import { CdkDragDrop, moveItemInArray } from '@angular/cdk/drag-drop';
import html2canvas from 'html2canvas';
import jsPDF from 'jspdf';

interface CountryGroup {
    country: string;
    panels: ResourcePanel[];
}

@Component({
    selector: 'ngx-resource-layout',
    templateUrl: './resource-layout.component.html',
    styleUrls: ['./resource-layout.component.scss']
})
export class ResourceLayoutComponent implements OnInit {

    selectedDept: string = 'all';
    timeRange: string = '1h';
    departments = [
        '组件研发组', '智能模型中心', '用户增长组', '研发组',
        '信贷平台部', '信贷策略中心', '唯渡', '数据开发组', '数据产品组',
        '商业分析组', '前端组', '决策平台部', '经营分析组', '后端组',
        '国内机构运营组', '国际业务组', '贷后业务组', '贷后策略部',
        '创新策略组', 'OA组'
    ];

    countries = ['China', 'Pakistan', 'Thailand', 'Philippines', 'Mexico', 'Indonesia', 'Common'];
    selectedCountry = 'China';
    dataSources: ResourceDataSource[] = [];

    exporting = false;

    // Selection for PDF
    sectionSelection: { [key: string]: boolean } = {
        department: true,
        application: true,
        cluster: true
    };

    // Summaries for each section
    summaries: { [key: string]: string } = {
        department: '',
        application: '',
        cluster: ''
    };

    // Custom range
    customStart: Date;
    customEnd: Date;

    // For inputs (formatted string)
    startTimeInput: string;
    endTimeInput: string;

    panels: ResourcePanel[] = [];

    // Grouped panels
    deptGroups: CountryGroup[] = [];
    appGroups: CountryGroup[] = [];
    clusterGroups: CountryGroup[] = [];

    @ViewChildren(ChartPanelComponent) chartPanels: QueryList<ChartPanelComponent>;

    constructor(
        private resourceService: ResourceService,
        private dialogService: NbDialogService,
        private toastrService: NbToastrService
    ) {
        // Load summaries from localStorage if available
        const saved = localStorage.getItem('resource_summaries');
        if (saved) {
            try {
                this.summaries = { ...this.summaries, ...JSON.parse(saved) };
            } catch (e) {
                console.error('Failed to parse summaries', e);
            }
        }
    }

    saveSummaries() {
        localStorage.setItem('resource_summaries', JSON.stringify(this.summaries));
    }

    ngOnInit() {
        this.loadPanels();
        this.setQuickRange('1h');
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
        this.deptGroups = this.groupPanels('department');
        this.appGroups = this.groupPanels('application');
        this.clusterGroups = this.groupPanels('cluster');
    }

    groupPanels(section: string): CountryGroup[] {
        const sectionPanels = this.panels.filter(p => p.section === section);
        const groups: CountryGroup[] = [];

        const targetCountries = (this.selectedCountry === 'All' || this.selectedCountry === '所有国家')
            ? this.countries
            : [this.selectedCountry];

        targetCountries.forEach(country => {
            const countryPanels = sectionPanels.filter(p => {
                const region = p.country || this.dataSources.find(d => d.id === p.data_source_id)?.region || 'China';
                return region === country;
            }).sort((a, b) => (a.display_order || 0) - (b.display_order || 0));

            if (countryPanels.length > 0) {
                groups.push({ country, panels: countryPanels });
            }
        });

        return groups;
    }

    onCountryChange() {
        this.applyFilters();
    }

    onFilterChange() {
        if (this.timeRange !== 'custom') {
            this.setQuickRange(this.timeRange);
        }
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

    editPanel(panel: ResourcePanel) {
        this.dialogService.open(PanelEditorComponent, {
            context: {
                panel: { ...panel, country: this.selectedCountry }, // copy
                isEdit: true
            },
            closeOnEsc: false,
            closeOnBackdropClick: false
        }).onClose.subscribe(res => {
            if (res) {
                // Update local panel via mutation to preserve object reference
                // This prevents ngOnChanges from triggering for this panel (and others)
                const index = this.panels.findIndex(p => p.id === res.id);
                if (index !== -1) {
                    Object.assign(this.panels[index], res);

                    // Re-apply filters to handle any group changes (e.g. region change)
                    this.applyFilters();

                    // Manually refresh the specific chart panel component
                    // Defer slightly to allow ngFor updates if any
                    setTimeout(() => {
                        const component = this.chartPanels.find(cp => cp.panel.id === res.id);
                        if (component) {
                            component.fetchData();
                        }
                    });
                } else {
                    this.loadPanels();
                }
            }
        });
    }

    addPanel(event: Event, section: string) {
        event.stopPropagation();

        // Find default data source for selected country? 
        // Just pick first matching one or let user choose.
        // We will pass empty panel with section.
        const defaultDs = this.dataSources.find(d => d.region === this.selectedCountry) || this.dataSources[0];

        this.dialogService.open(PanelEditorComponent, {
            context: {
                panel: {
                    section: section as any,
                    chart_type: 'line',
                    config: { width: 6 },
                    data_source_id: defaultDs?.id,
                    country: this.selectedCountry
                } as any,
                isEdit: false
            },
            closeOnEsc: false,
            closeOnBackdropClick: false
        }).onClose.subscribe(res => {
            if (res) this.loadPanels();
        });
    }

    deletePanel(panel: ResourcePanel) {
        if (confirm('Are you sure you want to delete this panel?')) {
            this.resourceService.deletePanel(panel.id).subscribe(() => {
                this.toastrService.success('Panel deleted', 'Success');
                this.loadPanels();
            });
        }
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
            await new Promise(resolve => setTimeout(resolve, 300));

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

            // Improved Capture Logic: Iterate through sections (accordion items)
            const items = Array.from(container.querySelectorAll('nb-accordion-item'));
            const targets: Element[] = [];

            items.forEach(item => {
                // Skip if section is hidden (via class d-none added in export mode)
                if (item.classList.contains('d-none')) return;

                const summary = item.querySelector('.summary-export-view.export-only');
                if (summary && summary.textContent?.trim()) {
                    targets.push(summary);
                }

                const groups = Array.from(item.querySelectorAll('.country-group-container'));
                targets.push(...groups);
            });

            if (targets.length === 0) {
                targets.push(container);
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
                        scale: 1,
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

            pdf.save(`resource-dashboard-${new Date().getTime()}.pdf`);
            this.toastrService.success('Export complete', 'PDF Export');

        } catch (e: any) {
            console.error(e);
            this.toastrService.danger(`Export failed: ${e.message}`, 'Error');
        } finally {
            container.classList.remove('exporting');
            this.exporting = false;
        }
    }

    drop(event: CdkDragDrop<ResourcePanel[]>, panels: ResourcePanel[]) {
        if (event.previousContainer === event.container) {
            moveItemInArray(panels, event.previousIndex, event.currentIndex);

            panels.forEach((p, index) => {
                p.display_order = index + 1;
            });

            panels.forEach(p => {
                this.resourceService.updatePanel(p.id, { display_order: p.display_order }).subscribe();
            });
        }
    }

    togglePanelWidth(panel: ResourcePanel) {
        const currentWidth = panel.config?.width || 6;
        const newWidth = currentWidth === 12 ? 6 : 12;
        if (!panel.config) panel.config = {};
        panel.config.width = newWidth;
        this.resourceService.updatePanel(panel.id, { config: panel.config }).subscribe();
    }

    async analyze(event: Event, section: string) {
        event.stopPropagation();
        if (section === 'application') return;

        this.toastrService.info(`Analyzing ${section} resources (local)...`, 'Smart Analysis');

        try {
            if (section === 'department') {
                this.analyzeDepartmentLocally();
            } else if (section === 'cluster') {
                this.analyzeClusterLocally();
            }
            this.toastrService.success(`${section} analysis complete`, 'Success');
            this.saveSummaries();
        } catch (e: any) {
            console.error(e);
            this.toastrService.danger(`Analysis failed: ${e.message || e}`, 'Error');
        }
    }

    private analyzeDepartmentLocally() {
        let summary = '';
        const panelsComponents = this.chartPanels.toArray();

        // Iterate groups
        this.deptGroups.forEach(group => {
            const country = group.country;
            if (this.deptGroups.length > 1) {
                summary += `\n=== ${country} ===\n`;
            }

            const groupPanelIds = new Set(group.panels.map(p => p.id));
            const groupComponents = panelsComponents.filter(cp => groupPanelIds.has(cp.panel.id));

            const findPanel = (part: string, exclude: string = '') => {
                const p = part.toLowerCase();
                const ex = exclude.toLowerCase();
                return groupComponents.find(cp => {
                    const title = (cp.panel.title || '').toLowerCase();
                    const query = (cp.panel.promql_query || '').toLowerCase();
                    const match = title.includes(p) || query.includes(p);
                    if (!match) return false;
                    if (ex && (title.includes(ex) || query.includes(ex))) return false;
                    return true;
                });
            };

            const cpuPanel = findPanel('cpu用量') || findPanel('avg_cpu_cores', '增长');
            const memPanel = findPanel('内存用量') || findPanel('avg_mem_bytes', '增长');
            const cpuGrowthPanel = findPanel('cpu增长率');
            const memGrowthPanel = findPanel('内存增长率');

            const perfPanel = findPanel('starrocks_audit_tbl__');
            const abnormalCpuPanel = findPanel('cpu开销');
            const abnormalMemPanel = findPanel('内存开销');
            const abnormalLatPanel = findPanel('执行时间查询');
            const abnormalRowPanel = findPanel('扫描行数查询');

            const fmtTop3 = (cp: ChartPanelComponent | undefined, stat: 'rawMax' | 'rawMean' | 'rawLast', unit: string, factor: number = 1) => {
                if (!cp || cp.legendItems.length === 0) return '无数据';
                return cp.legendItems
                    .filter(item => item.name !== '未知' && item[stat] !== undefined)
                    .slice()
                    .sort((a, b) => (b[stat] || 0) - (a[stat] || 0))
                    .slice(0, 3)
                    .map(item => {
                        const val = (item[stat] || 0) * factor;
                        const formatted = val.toFixed(Math.abs(val) > 10 ? 0 : 1);
                        return `${item.name} (${formatted}${unit})`;
                    })
                    .join('、');
            };

            const getExceptionCountStr = (cp: ChartPanelComponent | undefined) => {
                if (!cp) return '无数据';
                if (!cp.legendItems || cp.legendItems.length === 0) return '0';
                // If specific dept selected, find it
                if (this.selectedDept !== 'all') {
                    const item = cp.legendItems.find(i => i.name === this.selectedDept);
                    return item ? (item.rawSum || 0).toFixed(0) : '0';
                }
                return cp.legendItems
                    .filter(item => item.name !== '未知')
                    .slice()
                    .sort((a, b) => (b.rawSum || 0) - (a.rawSum || 0))
                    .slice(0, 5)
                    .map(item => `${item.name} (${(item.rawSum || 0).toFixed(0)}个)`)
                    .join('、');
            };

            const cpuPeakStr = fmtTop3(cpuPanel, 'rawMax', '核');
            const cpuAvgStr = fmtTop3(cpuPanel, 'rawMean', '核');
            const memPeakStr = fmtTop3(memPanel, 'rawMax', 'G', 1.0 / (1024 * 1024 * 1024));
            const memAvgStr = fmtTop3(memPanel, 'rawMean', 'G', 1.0 / (1024 * 1024 * 1024));
            const cpuGrowthStr = fmtTop3(cpuGrowthPanel, 'rawMean', '%');
            const memGrowthStr = fmtTop3(memGrowthPanel, 'rawMean', '%');

            let latPeakStr = '无数据';
            let latP99Str = '无数据';
            if (perfPanel && perfPanel.tableRows && perfPanel.tableRows.length > 0) {
                const rows = perfPanel.tableRows;
                const getCol = (row: any, ...keys: string[]) => {
                    for (const k of keys) if (row[k] !== undefined) return row[k];
                    return 0;
                };
                const filteredRows = rows.filter(r => (r['部门'] || r['department'] || '').toString() !== '未知');
                const sortedByPeak = [...filteredRows].sort((a, b) => getCol(b, '时延_峰值', 'lat_peak') - getCol(a, '时延_峰值', 'lat_peak'));
                const sortedByP99 = [...filteredRows].sort((a, b) => getCol(b, '时延_P99', 'lat_p99') - getCol(a, '时延_P99', 'lat_p99'));

                latPeakStr = sortedByPeak.slice(0, 3).map(r => `${r['部门'] || r['department']} (${getCol(r, '时延_峰值', 'lat_peak')}s)`).join('、');
                latP99Str = sortedByP99.slice(0, 3).map(r => `${r['部门'] || r['department']} (${getCol(r, '时延_P99', 'lat_p99')}s)`).join('、');
            }

            const excCpu = getExceptionCountStr(abnormalCpuPanel);
            const excMem = getExceptionCountStr(abnormalMemPanel);
            const excLat = getExceptionCountStr(abnormalLatPanel);
            const excRow = getExceptionCountStr(abnormalRowPanel);

            // Aggregate SQL Blacklist Top
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

            summary += `cpu 峰值top3：${cpuPeakStr}\n` +
                `cpu 平均top3：${cpuAvgStr}\n` +
                `内存峰值top3：${memPeakStr}\n` +
                `内存平均top3：${memAvgStr}\n` +
                `cpu增长率top3：${cpuGrowthStr}\n` +
                `内存增长率top3：${memGrowthStr}\n` +
                `时延峰值top3：${latPeakStr}\n` +
                `时延P99 top3：${latP99Str}\n` +
                `异常查询统计:\n` +
                ` - CPU开销: ${excCpu}\n` +
                ` - 内存开销: ${excMem}\n` +
                ` - 执行时间: ${excLat}\n` +
                ` - 扫描行数: ${excRow}\n` +
                `${blacklistStr}\n`;
        });

        this.summaries.department = summary;
    }

    private analyzeClusterLocally() {
        let summary = '';
        const panelsComponents = this.chartPanels.toArray();

        this.clusterGroups.forEach(group => {
            const country = group.country;
            if (this.clusterGroups.length > 1) {
                summary += `\n=== ${country} ===\n`;
            }
            const groupPanelIds = new Set(group.panels.map(p => p.id));
            const groupComponents = panelsComponents.filter(cp => groupPanelIds.has(cp.panel.id));
            const findPanel = (queryPart: string) => groupComponents.find(cp => cp.panel.promql_query.toLowerCase().includes(queryPart.toLowerCase()));

            const storagePanel = findPanel('starrocks_be_disks_data_used_capacity');
            const restartPanel = findPanel('up{group=~"be|fe"}');
            const rpsPanel = findPanel('starrocks_fe_request_total');
            const latPanel = findPanel('starrocks_fe_query_latency_ms');

            const getSingleValue = (cp: ChartPanelComponent | undefined) => {
                if (!cp) return '0';
                if (cp.statValue) return parseFloat(cp.statValue).toFixed(2);
                if (cp.legendItems.length > 0) return (cp.legendItems[0].rawLast || 0).toFixed(2);
                return '0';
            };

            const getStatsStr = (cp: ChartPanelComponent | undefined, unit: string = '') => {
                if (!cp || cp.legendItems.length === 0) return '无数据';
                const allLast = cp.legendItems.map(i => i.rawLast || 0);
                const max = Math.max(...allLast).toFixed(2);
                const min = Math.min(...allLast).toFixed(2);
                const avg = (allLast.reduce((a, b) => a + b, 0) / allLast.length).toFixed(2);
                return `${max} / ${min} / ${avg}${unit}`;
            };

            const storageStr = getSingleValue(storagePanel);
            const restartStr = getSingleValue(restartPanel);
            const rpsStr = getStatsStr(rpsPanel);
            const latStr = getStatsStr(latPanel, 's');

            summary += `存储使用率：${storageStr}%\n` +
                `组件稳定性：${restartStr}\n` +
                `RPS (Max/Min/Avg)：${rpsStr}\n` +
                `Latency (Max/Min/Avg)：${latStr}\n`;
        });

        this.summaries.cluster = summary;
    }
    trackByGroup(index: number, item: CountryGroup) {
        return item.country;
    }

    trackByPanel(index: number, item: ResourcePanel) {
        return item.id;
    }
}

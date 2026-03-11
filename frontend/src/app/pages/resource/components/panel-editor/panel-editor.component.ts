import { Component, Input, OnInit } from '@angular/core';
import { NbDialogRef } from '@nebular/theme';
import { ResourceService, ResourcePanel, ResourceDataSource } from '../../../../@core/services/resource.service';

@Component({
    selector: 'ngx-panel-editor',
    templateUrl: './panel-editor.component.html',
    styleUrls: ['./panel-editor.component.scss']
})
export class PanelEditorComponent implements OnInit {
    @Input() isEdit: boolean = false;
    @Input() section?: string;
    @Input() panel?: ResourcePanel;

    model: Partial<ResourcePanel> = {
        chart_type: 'line',
        promql_query: ''
    };

    dataSources: ResourceDataSource[] = [];

    showLast: boolean = false;

    // Unpacked config
    legendFormat: string = '';
    unit: string = '';
    showMax: boolean = false;
    showMin: boolean = false;
    showMean: boolean = false;

    get selectedDataSourceType(): string {
        const ds = this.dataSources.find(d => d.id === this.model.data_source_id);
        return ds ? ds.type : '';
    }

    get isSqlSource(): boolean {
        const type = this.selectedDataSourceType;
        return type === 'mysql' || type === 'starrocks';
    }

    get filteredDataSources(): ResourceDataSource[] {
        if (this.dataSources.length === 0) return [];
        let country = '';
        if (this.isEdit) {
            // Priority: panel's own country field
            country = this.model.country || '';
            // If missing, infer from existing data source
            if (!country) {
                const currentDs = this.dataSources.find(d => d.id === this.model.data_source_id);
                if (currentDs) country = currentDs.region || 'China';
            }
        } else {
            // New panel, use passed-in country context
            country = (this.model as any).country || 'China';
        }

        if (!country || country === 'All' || country === '所有国家') return this.dataSources;
        return this.dataSources.filter(ds => ds.region === country || ds.region === 'Common');
    }

    onDataSourceChange() {
        if (this.isSqlSource) {
            if (this.model.chart_type !== 'table' && this.model.chart_type !== 'pie' &&
                this.model.chart_type !== 'line' && this.model.chart_type !== 'bar') {
                this.model.chart_type = 'table';
            }
        } else if (this.model.chart_type === 'table') {
            this.model.chart_type = 'line';
        }
    }

    constructor(
        protected ref: NbDialogRef<PanelEditorComponent>,
        private resourceService: ResourceService
    ) { }

    ngOnInit() {
        this.resourceService.getDataSources().subscribe(data => {
            this.dataSources = data;
            if (!this.isEdit && !this.model.data_source_id && data.length > 0) {
                this.model.data_source_id = data[0].id; // Default new panel to first source
            }
            this.onDataSourceChange();
        });

        if (this.isEdit && this.panel) {
            this.model = { ...this.panel };
            if (this.panel.config) {
                this.legendFormat = this.panel.config.legendFormat || '';
                this.unit = this.panel.config.unit || '';
                this.showMax = !!this.panel.config.showMax;
                this.showMin = !!this.panel.config.showMin;
                this.showMean = !!this.panel.config.showMean;
                this.showLast = !!this.panel.config.showLast;
            }
        } else {
            if (this.panel) {
                this.model = { ...this.model, ...this.panel };
            }
            if (this.section) {
                this.model.section = this.section as any;
            }
        }
    }

    cancel() {
        this.ref.close();
    }

    submit() {
        // Pack config
        this.model.config = {
            legendFormat: this.legendFormat,
            unit: this.unit,
            showMax: this.showMax,
            showMin: this.showMin,
            showMean: this.showMean,
            showLast: this.showLast
        };

        if (this.isEdit && this.panel) {
            this.resourceService.updatePanel(this.panel.id, this.model).subscribe(res => {
                this.ref.close(res);
            });
        } else {
            this.resourceService.createPanel(this.model).subscribe(res => {
                this.ref.close(res);
            });
        }
    }
}

import { Component, OnInit } from '@angular/core';
import { NbDialogService, NbToastrService } from '@nebular/theme';
import { ResourceService, ResourceDataSource } from '../../../../@core/services/resource.service';
import { DataSourceDialogComponent } from '../datasource-dialog/datasource-dialog.component';

@Component({
    selector: 'ngx-datasource-list',
    templateUrl: './datasource-list.component.html',
    styleUrls: ['./datasource-list.component.scss']
})
export class DataSourceListComponent implements OnInit {
    dataSources: ResourceDataSource[] = [];
    groupedDataSources: { [key: string]: ResourceDataSource[] } = {};
    regionKeys: string[] = [];

    constructor(
        private resourceService: ResourceService,
        private dialogService: NbDialogService,
        private toastr: NbToastrService
    ) { }

    ngOnInit() {
        this.load();
    }

    getIconPath(type: string): string {
        return `/assets/images/datasource/${type}.png`;
    }

    load() {
        this.resourceService.getDataSources().subscribe(data => {
            this.dataSources = data;
            this.groupDataSources();
        });
    }

    groupDataSources() {
        this.groupedDataSources = {};
        this.dataSources.forEach(ds => {
            const region = ds.region || 'China'; // Default to China
            if (!this.groupedDataSources[region]) {
                this.groupedDataSources[region] = [];
            }
            this.groupedDataSources[region].push(ds);
        });
        // Custom sort: 'Common' first, then alphabetical
        this.regionKeys = Object.keys(this.groupedDataSources).sort((a, b) => {
            if (a === 'Common') return -1;
            if (b === 'Common') return 1;
            return a.localeCompare(b);
        });
    }

    edit(ds?: ResourceDataSource) {
        this.dialogService.open(DataSourceDialogComponent, {
            context: { isEdit: !!ds, dataSource: ds }
        }).onClose.subscribe(res => {
            if (res) this.load();
        });
    }

    delete(ds: ResourceDataSource) {
        if (confirm(`Delete data source ${ds.name}?`)) {
            this.resourceService.deleteDataSource(ds.id!).subscribe(() => {
                this.toastr.success('Deleted', 'Success');
                this.load();
            });
        }
    }

    hasMappings(mapping: any): boolean {
        return mapping && Object.keys(mapping).length > 0;
    }

    getMappingCount(mapping: any): number {
        return mapping ? Object.keys(mapping).length : 0;
    }
}

import { Component, Input, OnInit } from '@angular/core';
import { NbDialogRef, NbToastrService } from '@nebular/theme';
import { ResourceService, ResourceDataSource } from '../../../../@core/services/resource.service';

@Component({
    selector: 'ngx-datasource-dialog',
    templateUrl: './datasource-dialog.component.html',
    styleUrls: ['./datasource-dialog.component.scss']
})
export class DataSourceDialogComponent implements OnInit {
    @Input() isEdit: boolean = false;
    @Input() dataSource?: ResourceDataSource;

    model: Partial<ResourceDataSource> = {
        type: 'prometheus',
        connection_timeout: 10
    };

    testing = false;
    mappingEntries: { key: string, value: string }[] = [];

    constructor(
        protected ref: NbDialogRef<DataSourceDialogComponent>,
        private resourceService: ResourceService,
        private toastr: NbToastrService
    ) { }

    ngOnInit() {
        if (this.isEdit && this.dataSource) {
            this.model = { ...this.dataSource };
            // Don't prefill password
            this.model.password = '';

            if (this.model.connection_timeout === undefined || this.model.connection_timeout === null) {
                this.model.connection_timeout = 10;
            }

            // Initial mapping entries from JSON
            if (this.model.fe_mapping) {
                this.mappingEntries = Object.entries(this.model.fe_mapping).map(([key, value]) => ({
                    key,
                    value: value as string
                }));
            }
        }
    }

    addMapping() {
        this.mappingEntries.push({ key: '', value: '' });
    }

    removeMapping(index: number) {
        this.mappingEntries.splice(index, 1);
    }

    test() {
        if (!this.model.name || !this.model.url) {
            this.toastr.warning('Please fill in Name and URL', 'Validation');
            return;
        }

        this.testing = true;

        // Prepare mapping for test too
        const mappingObj: any = {};
        this.mappingEntries.forEach(e => {
            if (e.key && e.value) mappingObj[e.key] = e.value;
        });

        const data = {
            id: this.dataSource?.id,
            ...this.model,
            fe_mapping: mappingObj
        };

        this.resourceService.testDataSource(data).subscribe({
            next: (res) => {
                this.testing = false;
                if (res.status === 'success') {
                    this.toastr.success(res.message, 'Success');
                } else {
                    this.toastr.danger(res.message, 'Failed');
                }
            },
            error: (err) => {
                this.testing = false; // Fixed status assignment
                this.toastr.danger(err.error?.message || 'Unknown error', 'Failed');
            }
        });
    }

    cancel() {
        this.ref.close();
    }

    submit() {
        // Convert entries back to JSON
        const mappingObj: any = {};
        this.mappingEntries.forEach(e => {
            if (e.key && e.value) {
                mappingObj[e.key] = e.value;
            }
        });
        this.model.fe_mapping = mappingObj;

        if (this.isEdit && this.dataSource) {
            const payload = { ...this.model };
            if (!payload.password) delete payload.password;

            this.resourceService.updateDataSource(this.dataSource.id!, payload as ResourceDataSource).subscribe(res => {
                this.ref.close(res);
            });
        } else {
            this.resourceService.createDataSource(this.model as ResourceDataSource).subscribe(res => {
                this.ref.close(res);
            });
        }
    }
}

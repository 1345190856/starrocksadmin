import { Component, Input, OnInit } from '@angular/core';
import { NbDialogRef } from '@nebular/theme';
import { ResourceAsset } from '../../../@core/services/asset.service';

@Component({
    selector: 'ngx-asset-edit-dialog',
    templateUrl: './asset-edit-dialog.component.html',
    styleUrls: ['./asset-edit-dialog.component.scss'],
})
export class AssetEditDialogComponent implements OnInit {
    @Input() mode: 'add' | 'edit' = 'edit';
    @Input() resource: ResourceAsset;
    @Input() filterOptions: any = {
        project_names: [],
        service_types: [],
        regions: [],
    };

    items: any[] = [{}];

    constructor(protected ref: NbDialogRef<AssetEditDialogComponent>) { }

    ngOnInit() {
        if (this.mode === 'edit' && this.resource) {
            this.items = [{ ...this.resource }];
        } else {
            this.items = [{}];
        }
    }

    addRow() {
        this.items.push({ manual_service: '' });
    }

    removeRow(index: number) {
        if (this.items.length > 1) {
            this.items.splice(index, 1);
        }
    }

    addServiceTag(item: any, tagInput: HTMLInputElement) {
        const val = tagInput.value.trim();
        if (val) {
            const currentTags = this.getServiceTags(item.manual_service);
            if (!currentTags.includes(val)) {
                currentTags.push(val);
                item.manual_service = currentTags.join(',');
            }
            tagInput.value = '';
        }
    }

    removeServiceTag(item: any, index: number) {
        const tags = this.getServiceTags(item.manual_service);
        tags.splice(index, 1);
        item.manual_service = tags.join(',');
    }

    getServiceTags(manualService: string): string[] {
        if (!manualService) return [];
        return manualService.split(',').map(s => s.trim()).filter(s => s !== '');
    }

    cancel() {
        this.ref.close();
    }

    submit() {
        // Validate items: Instance Type and Private IP are required
        const validItems = this.items.filter(item => item.instance_type && item.private_ip);
        if (validItems.length === 0) {
            return;
        }

        // Ensure numeric fields are strings
        const processedItems = validItems.map(item => {
            const newItem = { ...item };
            ['cpu', 'memory', 'storage', 'public_ip', 'private_ip', 'instance_id'].forEach(field => {
                if (typeof newItem[field] === 'number') {
                    newItem[field] = newItem[field].toString();
                }
            });
            return newItem;
        });

        if (this.mode === 'edit') {
            this.ref.close(processedItems[0]);
        } else {
            this.ref.close(processedItems);
        }
    }
}

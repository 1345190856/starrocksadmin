import { Component, OnInit } from '@angular/core';
import { NbDialogService, NbToastrService } from '@nebular/theme';
import { LocalDataSource } from 'ng2-smart-table';
import { DutyService, DutyPersonnel, DutySchedule } from '../../../@core/services/duty.service';
import { DutyPersonnelDialogComponent } from './personnel-dialog/personnel-dialog.component';
import { DutyWheelComponent } from './duty-wheel/duty-wheel.component';
import { ConfirmDialogService } from '../../../@core/services/confirm-dialog.service';
import { NotifyDialogComponent } from '../notify-dialog/notify-dialog.component';
import { AlertService } from '../../../@core/services/alert.service';

@Component({
    selector: 'ngx-duty-personnel',
    templateUrl: './personnel.component.html',
    styleUrls: ['./personnel.component.scss']
})
export class DutyPersonnelComponent implements OnInit {
    settings = {
        hideSubHeader: true,
        actions: {
            add: false,
            edit: true,
            delete: true,
            position: 'right',
            columnTitle: 'Actions',
        },
        edit: {
            editButtonContent: '<i class="nb-edit"></i>',
        },
        delete: {
            deleteButtonContent: '<i class="nb-trash"></i>',
        },
        mode: 'external',
        columns: {
            name: { title: '姓名', type: 'string' },
            org_lvl1: { title: '一级组织', type: 'string' },
            org_lvl2: { title: '二级组织', type: 'string' },
            email: { title: '邮箱', type: 'string' },
            phone: { title: '电话', type: 'string' },
            duty_platform: { title: '值班平台', type: 'string' },
            responsible_domain: {
                title: '负责域',
                type: 'html',
                valuePrepareFunction: (value) => {
                    if (!value) return '';
                    return value.split(',').map(tag => `<span class="service-tag">${tag.trim()}</span>`).join('');
                }
            },
            countries: {
                title: '国家',
                type: 'html',
                valuePrepareFunction: (value) => {
                    if (!value) return '';
                    return value.split(',').map(tag => `<span class="service-tag">${tag.trim()}</span>`).join('');
                }
            },
        },
    };

    source: LocalDataSource = new LocalDataSource();

    constructor(
        private dutyService: DutyService,
        private dialogService: NbDialogService,
        private toastrService: NbToastrService,
        private confirmService: ConfirmDialogService,
        private alertService: AlertService
    ) { }

    ngOnInit() {
        this.loadData();
    }

    loadData() {
        this.dutyService.getPersonnel().subscribe({
            next: (data) => this.source.load(data),
            error: (err) => this.toastrService.danger('Could not load personnel', 'Error')
        });
    }

    onCreate() {
        this.dialogService.open(DutyPersonnelDialogComponent)
            .onClose.subscribe(res => {
                if (res) this.loadData();
            });
    }

    onOpenWheel() {
        this.dutyService.getPersonnel().subscribe(data => {
            this.dialogService.open(DutyWheelComponent, {
                context: { personnel: data }
            }).onClose.subscribe(res => {
                if (res) this.loadData();
            });
        });
    }

    onEdit(event: any) {
        console.log('Editing personnel row data:', event.data);
        this.dialogService.open(DutyPersonnelDialogComponent, {
            context: { personnel: event.data }
        }).onClose.subscribe(res => {
            if (res) this.loadData();
        });
    }

    onDelete(event: any) {
        this.confirmService.confirmDelete(event.data.name).subscribe(confirmed => {
            if (confirmed) {
                this.dutyService.deletePersonnel(event.data.id).subscribe({
                    next: () => {
                        this.toastrService.success('Deleted successfully', 'Success');
                        this.loadData();
                    },
                    error: () => this.toastrService.danger('Delete failed', 'Error')
                });
            }
        });
    }

    // Notification Logic
    notifyDuty() {
        this.dialogService.open(NotifyDialogComponent, {
            context: {
                platformName: '所有平台'
            }
        }).onClose.subscribe(botIds => {
            if (!botIds || botIds.length === 0) return;

            this.dutyService.notifyManual(botIds).subscribe({
                next: () => {
                    this.toastrService.success(`值班信息已发送至 ${botIds.length} 个通道`, '成功');
                },
                error: (err) => {
                    console.error('Failed to send notification:', err);
                    this.toastrService.danger('发送值班通知失败', '错误');
                }
            });
        });
    }

    private formatDate(d: Date): string {
        const year = d.getFullYear();
        const month = ('0' + (d.getMonth() + 1)).slice(-2);
        const day = ('0' + d.getDate()).slice(-2);
        return `${year}-${month}-${day}`;
    }

    private translateCountry(country: string): string {
        const mapping: { [key: string]: string } = {
            'China': '中国',
            'Philippines': '菲律宾',
            'Mexico': '墨西哥',
            'Pakistan': '巴基斯坦',
            'Indonesia': '印尼',
            'Thailand': '泰国'
        };
        return mapping[country] || country;
    }
}

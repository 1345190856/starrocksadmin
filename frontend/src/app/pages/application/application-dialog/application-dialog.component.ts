import { Component, Input, OnInit } from '@angular/core';
import { NbDialogRef, NbToastrService } from '@nebular/theme';
import { Application, ApplicationService } from '../../../@core/services/application.service';

@Component({
    selector: 'ngx-application-dialog',
    templateUrl: './application-dialog.component.html',
    styleUrls: ['./application-dialog.component.scss']
})
export class ApplicationDialogComponent implements OnInit {
    @Input() isEdit: boolean = false;
    @Input() application: Application;

    types = ['prometheus', 'mysql', 'grafana', 'jenkins', 'DolphinScheduler', 'superset', 'starrocks', 'metabase', 'application', 'jumpserver'];
    regions = ['常用工具', '中国', '巴基斯坦', '印尼', '菲律宾', '泰国', '墨西哥'];

    loading = false;

    constructor(
        protected ref: NbDialogRef<ApplicationDialogComponent>,
        private applicationService: ApplicationService,
        private toastr: NbToastrService
    ) { }

    ngOnInit() {
        if (!this.application) {
            this.application = { name: '', type: 'prometheus', address: '', region: '常用工具' };
        }
        this.checkType();
    }

    onRegionChange(region: string) {
        this.application.region = region;
        this.checkType();
    }

    checkType() {
        if (this.application.region === '常用工具') {
            this.application.type = 'application';
        }
    }

    cancel() {
        this.ref.close();
    }

    submit() {
        if (!this.application.name || !this.application.address) {
            this.toastr.warning('请填写完整信息', '提示');
            return;
        }

        this.loading = true;
        if (this.isEdit) {
            this.applicationService.updateApplication(this.application.id!, this.application).subscribe({
                next: () => {
                    this.toastr.success('更新成功', '成功');
                    this.ref.close(true);
                },
                error: () => this.loading = false
            });
        } else {
            this.applicationService.createApplication(this.application).subscribe({
                next: () => {
                    this.toastr.success('创建成功', '成功');
                    this.ref.close(true);
                },
                error: () => this.loading = false
            });
        }
    }
}

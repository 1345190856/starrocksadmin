import { Component, OnInit } from '@angular/core';
import { NbDialogService, NbToastrService } from '@nebular/theme';
import { Application, ApplicationService } from '../../../@core/services/application.service';
import { ApplicationDialogComponent } from '../application-dialog/application-dialog.component';

@Component({
    selector: 'ngx-application-list',
    templateUrl: './application-list.component.html',
    styleUrls: ['./application-list.component.scss']
})
export class ApplicationListComponent implements OnInit {
    regions = ['常用工具', '中国', '巴基斯坦', '印尼', '菲律宾', '泰国', '墨西哥'];
    applications: Application[] = [];
    groupedApplications: { [key: string]: Application[] } = {};

    constructor(
        private applicationService: ApplicationService,
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
        this.applicationService.getApplications().subscribe(data => {
            this.applications = data;
            this.groupApplications();
        });
    }

    groupApplications() {
        this.groupedApplications = {};
        this.regions.forEach(region => {
            this.groupedApplications[region] = this.applications.filter(app => app.region === region);
        });
    }

    add(region: string) {
        const type = region === '常用工具' ? 'application' : 'prometheus';
        this.dialogService.open(ApplicationDialogComponent, {
            context: { isEdit: false, application: { region, name: '', type, address: '' } }
        }).onClose.subscribe(res => {
            if (res) this.load();
        });
    }

    edit(app: Application) {
        this.dialogService.open(ApplicationDialogComponent, {
            context: { isEdit: true, application: { ...app } }
        }).onClose.subscribe(res => {
            if (res) this.load();
        });
    }

    delete(app: Application) {
        if (confirm(`确定删除应用 ${app.name}?`)) {
            this.applicationService.deleteApplication(app.id!).subscribe(() => {
                this.toastr.success('删除成功', '成功');
                this.load();
            });
        }
    }

    openUrl(address: string) {
        if (!address.startsWith('http')) {
            address = 'http://' + address;
        }
        window.open(address, '_blank');
    }
}

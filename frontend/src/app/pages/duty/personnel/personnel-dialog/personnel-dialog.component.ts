import { Component, Input, OnInit, ChangeDetectorRef } from '@angular/core';
import { NbDialogRef } from '@nebular/theme';
import { DutyService, DutyPersonnel } from '../../../../@core/services/duty.service';
import { Observable, of } from 'rxjs';
import { map } from 'rxjs/operators';
import { HeadcountService, Employee } from '../../../../@core/services/headcount.service';

@Component({
    selector: 'ngx-duty-personnel-dialog',
    templateUrl: './personnel-dialog.component.html',
    styleUrls: ['./personnel-dialog.component.scss']
})
export class DutyPersonnelDialogComponent implements OnInit {
    private _personnel?: DutyPersonnel;
    @Input() set personnel(val: DutyPersonnel | undefined) {
        this._personnel = val;
        if (val) {
            this.mapPersonnelToModel(val);
        }
    }
    get personnel(): DutyPersonnel | undefined {
        return this._personnel;
    }

    // Form model
    model: Partial<DutyPersonnel> = {
        name: '',
        org_lvl1: '',
        org_lvl2: '',
        email: '',
        phone: '',
        duty_platform: '',
        responsible_domain: '',
        countries: '',
    };

    filteredEmployees$: Observable<Employee[]>;
    currentEmployees: Employee[] = [];

    platforms = ['数仓', '数据平台', '无'];
    allCountries = ['all', '无', '中国', '墨西哥', '巴基斯坦', '印尼', '泰国', '菲律宾'];
    selectedCountries: string[] = [];

    constructor(
        protected ref: NbDialogRef<DutyPersonnelDialogComponent>,
        private dutyService: DutyService,
        private headcountService: HeadcountService,
        private cdr: ChangeDetectorRef
    ) { }

    ngOnInit() {
        this.filteredEmployees$ = of([]);
        // Double check if data was passed via context
        if (this.personnel) {
            this.mapPersonnelToModel(this.personnel);
        }
    }

    mapPersonnelToModel(val: DutyPersonnel) {
        console.log('Mapping personnel to model:', val);
        this.model = { ...val };

        // Ensure strings for all fields to avoid UI glitches
        this.model.name = val.name || '';
        this.model.email = val.email || '';
        this.model.phone = val.phone || '';
        this.model.org_lvl1 = val.org_lvl1 || '';
        this.model.org_lvl2 = val.org_lvl2 || '';
        this.model.duty_platform = val.duty_platform || '';
        this.model.responsible_domain = val.responsible_domain || '';

        if (val.countries) {
            this.selectedCountries = val.countries.split(',').filter(c => !!c);
        } else {
            this.selectedCountries = [];
        }

        // Trigger change detection with a small delay to ensure template is ready
        setTimeout(() => {
            if (this.cdr && !this.cdr['destroyed']) {
                this.cdr.detectChanges();
            }
        }, 50);
    }

    handleDisplayFn = (employee: any): string => {
        if (!employee) return '';
        if (typeof employee === 'string') return employee;
        return employee.name || '';
    }

    onNameInput(value: string) {
        if (!value || value.length < 1) {
            this.filteredEmployees$ = of([]);
            this.currentEmployees = [];
            return;
        }
        this.headcountService.listEmployees(1, 10, value).subscribe(res => {
            this.currentEmployees = res.data.list;
            this.filteredEmployees$ = of(this.currentEmployees);
        });
    }

    onEmployeeSelected(value: any) {
        // If it's a string (name from option value), find the employee object
        if (typeof value === 'string') {
            const employee = this.currentEmployees.find(e => e.name === value);
            if (employee) {
                this.fillFromEmployee(employee);
            } else {
                this.model.name = value;
            }
        } else if (value && value.name) {
            // If it's the whole object
            this.fillFromEmployee(value);
        }
    }

    private fillFromEmployee(employee: Employee) {
        this.model.name = employee.name;
        this.model.org_lvl1 = employee.orgName_2 || '';
        this.model.org_lvl2 = employee.orgName || '';
        this.model.email = employee.email || '';
        this.model.phone = employee.phone || '';
        this.cdr.detectChanges();
    }

    // Responsible Domain Tag Management
    getDomainTags(): string[] {
        if (!this.model.responsible_domain) return [];
        return this.model.responsible_domain.split(',').map(s => s.trim()).filter(s => !!s);
    }

    addDomainTag(tagInput: HTMLInputElement) {
        const val = tagInput.value.trim();
        if (val) {
            const currentTags = this.getDomainTags();
            if (!currentTags.includes(val)) {
                currentTags.push(val);
                this.model.responsible_domain = currentTags.join(',');
            }
            tagInput.value = '';
        }
    }

    removeDomainTag(index: number) {
        const tags = this.getDomainTags();
        tags.splice(index, 1);
        this.model.responsible_domain = tags.join(',');
    }

    cancel() {
        this.ref.close();
    }

    submit() {
        if (!this.model.name || !this.model.email) {
            return;
        }

        this.model.countries = this.selectedCountries.join(',');

        if (this.personnel && this.personnel.id) {
            this.dutyService.updatePersonnel(this.personnel.id, this.model).subscribe(res => {
                this.ref.close(res);
            });
        } else {
            this.dutyService.createPersonnel(this.model).subscribe(res => {
                this.ref.close(res);
            });
        }
    }
}

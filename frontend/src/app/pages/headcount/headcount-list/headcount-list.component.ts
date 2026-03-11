import { Component, OnInit } from '@angular/core';
import { HeadcountService, Employee } from '../../../@core/services/headcount.service';
import { NbToastrService } from '@nebular/theme';

@Component({
    selector: 'ngx-headcount-list',
    templateUrl: './headcount-list.component.html',
    styleUrls: ['./headcount-list.component.scss'],
})
export class HeadcountListComponent implements OnInit {
    isLoading = false;
    isSyncing = false;
    employees: Employee[] = [];
    total = 0;

    // Pagination
    page = 1;
    pageSize = 20;
    pageSizes = [20, 50, 100, 200];

    // Sorting
    sortBy = '';
    sortOrder: 'asc' | 'desc' = 'asc';

    // Filter
    query = '';

    // Columns
    allColumns = [
        { key: 'id', label: 'ID', sortable: true },
        { key: 'userId', label: 'User ID', default: true },
        { key: 'name', label: 'Name', default: true },
        { key: 'city', label: 'City', default: true },
        { key: 'company', label: 'Company', default: true },
        { key: 'email', label: 'Email', default: true },
        { key: 'employeeNumber', label: 'Employee No' },
        { key: 'phone', label: 'Phone', default: true },
        { key: 'orgFullPath', label: 'Org Path', default: true },
        { key: 'position', label: 'Position', default: true },
        { key: 'position_level_mame', label: 'Level', default: true, sortable: true },
        { key: 'labor_type_txt', label: 'Labor Type', default: true },
        { key: 'status_txt', label: 'Status', default: true, sortable: true },
        { key: 'join_at', label: 'Join Date', sortable: true },
        { key: 'leaveAt', label: 'Leave Date', sortable: true },
        { key: 'orgName', label: 'Org Name' },
        { key: 'orgName_1', label: 'Org Level 1' },
        { key: 'orgName_2', label: 'Org Level 2' },
        { key: 'leaderEmployeeNumber', label: 'Leader No' },
        // { key: 'leaderId', label: 'Leader ID' },
    ];

    selectedColumns: Set<string> = new Set();

    constructor(
        private headcountService: HeadcountService,
        private toastrService: NbToastrService,
    ) {
        // Initialize default columns
        this.allColumns.forEach(col => {
            if (col.default) {
                this.selectedColumns.add(col.key);
            }
        });
    }

    ngOnInit(): void {
        this.loadData();
    }

    loadData(): void {
        this.isLoading = true;
        this.headcountService.listEmployees(this.page, this.pageSize, this.query, this.sortBy, this.sortOrder).subscribe({
            next: (res) => {
                if (res.code === 0) {
                    this.employees = res.data.list;
                    this.total = res.data.total;
                } else {
                    this.toastrService.danger(res.message, 'Error');
                }
                this.isLoading = false;
            },
            error: (err) => {
                this.toastrService.danger(err.message, 'Error');
                this.isLoading = false;
            },
        });
    }

    onSort(key: string): void {
        const column = this.allColumns.find(c => c.key === key);
        if (!column || !column.sortable) return;

        if (this.sortBy === key) {
            this.sortOrder = this.sortOrder === 'asc' ? 'desc' : 'asc';
        } else {
            this.sortBy = key;
            this.sortOrder = 'asc';
        }
        this.page = 1;
        this.loadData();
    }

    onSearch(): void {
        this.page = 1;
        this.loadData();
    }

    onPageChange(page: number): void {
        this.page = page;
        this.loadData();
    }

    onPageSizeChange(pageSize: number): void {
        this.pageSize = pageSize;
        this.page = 1;
        this.loadData();
    }

    onSync(): void {
        this.isSyncing = true;
        this.headcountService.syncEmployees().subscribe({
            next: (res) => {
                if (res.code === 0) {
                    this.toastrService.success(res.data, 'Success');
                    this.loadData();
                } else {
                    this.toastrService.danger(res.message, 'Sync Failed');
                }
                this.isSyncing = false;
            },
            error: (err) => {
                this.toastrService.danger(err.message, 'Sync Failed');
                this.isSyncing = false;
            },
        });
    }

    toggleColumn(key: string, checked: boolean): void {
        if (checked) {
            this.selectedColumns.add(key);
        } else {
            this.selectedColumns.delete(key);
        }
    }

    get pages(): (number | string)[] {
        const totalPages = Math.ceil(this.total / this.pageSize);
        if (totalPages === 0) return [1];

        if (totalPages <= 7) {
            return Array.from({ length: totalPages }, (_, i) => i + 1);
        }

        const pages: (number | string)[] = [1];

        if (this.page > 4) {
            pages.push('...');
        }

        let start = Math.max(2, this.page - 2);
        let end = Math.min(totalPages - 1, this.page + 2);

        if (this.page <= 4) {
            end = 5;
            start = 2;
        } else if (this.page >= totalPages - 3) {
            start = totalPages - 4;
            end = totalPages - 1;
        }

        for (let i = start; i <= end; i++) {
            pages.push(i);
        }

        if (this.page < totalPages - 3) {
            pages.push('...');
        }

        pages.push(totalPages);

        return pages;
    }

    goToPage(p: number | string): void {
        if (typeof p === 'number') {
            this.onPageChange(p);
        }
    }

    isColumnVisible(key: string): boolean {
        return this.selectedColumns.has(key);
    }
}

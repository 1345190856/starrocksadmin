import { Component, OnInit, ViewChild, TemplateRef } from '@angular/core';
import { AssetService, ResourceAsset } from '../../../@core/services/asset.service';
import { NbToastrService, NbDialogService } from '@nebular/theme';
import * as XLSX from 'xlsx';
import { AssetEditDialogComponent } from '../asset-edit-dialog/asset-edit-dialog.component';
import { ConfirmDialogService } from '../../../@core/services/confirm-dialog.service';
import { SystemService } from '../../../@core/services/system.service';

@Component({
    selector: 'ngx-asset-inventory-list',
    templateUrl: './asset-inventory-list.component.html',
    styleUrls: ['./asset-inventory-list.component.scss'],
})
export class AssetInventoryListComponent implements OnInit {
    isLoading = false;
    isImporting = false;
    resources: ResourceAsset[] = [];
    total = 0;
    isApplying = false;
    applyForm = {
        ipList: '',
        cookie: '',
        remarks: '申请机器权限进行日常运维，请领导审批。',
    };

    // Selection
    selectedIps: Set<string> = new Set();

    // Pagination
    page = 1;
    pageSize = 20;
    pageSizes = [20, 50, 100, 200];

    // Filters
    searchQuery = '';
    filterInstanceType = '';
    filterProjectName = '';
    filterServiceType = '';
    filterCountry = '';
    filterRegion = '';
    filterStatus = '';
    filterServiceStatus = '';

    filterOptions: any = {
        project_names: [],
        service_types: [],
        service_statuses: [],
        countries: [],
        regions: [],
    };

    allColumns = [
        { key: 'instance_type', label: '实例类型', default: true },
        { key: 'instance_name', label: '实例名称', default: true },
        { key: 'instance_id', label: '实例编号' },
        { key: 'private_ip', label: '内网IP', default: true },
        { key: 'public_ip', label: 'EIP' },
        { key: 'manual_service', label: '服务类型', default: true },
        { key: 'cpu', label: 'CPU', default: true },
        { key: 'memory', label: '内存', default: true },
        { key: 'storage', label: '存储', default: true },
        { key: 'network_identifier', label: '网络标识' },
        { key: 'release', label: '发行版本', default: true },
        { key: 'country', label: '国家', default: true },
        { key: 'region', label: '地区' },
        { key: 'project_name', label: '项目名称' },
        { key: 'project_ownership', label: '项目归属' },
        { key: 'created_at', label: '创建时间' },
        { key: 'remark', label: '备注' },
    ];

    selectedColumns: Set<string> = new Set();

    @ViewChild('configDialog') configDialog: TemplateRef<any>;
    @ViewChild('applyResultDialog') applyResultDialog: TemplateRef<any>;
    @ViewChild('serviceLogDialog') serviceLogDialog: TemplateRef<any>;
    webhookUrl = '';

    // Service Op State
    currentService = '';
    currentIp = '';
    serviceLogContent = '';
    isOpLoading = false;

    constructor(
        private assetService: AssetService,
        private toastrService: NbToastrService,
        private dialogService: NbDialogService,
        private confirmDialogService: ConfirmDialogService,
        private systemService: SystemService,
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
        this.loadFilterOptions();
    }

    loadFilterOptions(): void {
        this.assetService.getFilterOptions().subscribe({
            next: (res) => {
                if (res.code === 0) {
                    this.filterOptions = res.data;
                }
            },
        });
    }

    loadData(): void {
        this.isLoading = true;
        this.selectedIps.clear();
        const params = {
            page: this.page,
            page_size: this.pageSize,
            query: this.searchQuery,
            instance_type: this.filterInstanceType,
            project_name: this.filterProjectName,
            manual_service: this.filterServiceType,
            country: this.filterCountry,
            status: this.filterStatus,
            region: this.filterRegion,
            service_status: this.filterServiceStatus,
        };

        this.assetService.listResources(params).subscribe({
            next: (res) => {
                if (res.code === 0) {
                    this.resources = res.data.list;
                    this.total = res.data.total;
                } else {
                    this.toastrService.danger(res.message, '加载失败');
                }
                this.isLoading = false;
            },
            error: (err) => {
                this.toastrService.danger(err.message, '加载失败');
                this.isLoading = false;
            },
        });
    }

    onSearch(): void {
        this.page = 1;
        this.loadData();
    }

    onReset(): void {
        this.searchQuery = '';
        this.filterInstanceType = '';
        this.filterProjectName = '';
        this.filterServiceType = '';
        this.filterCountry = '';
        this.filterRegion = '';
        this.filterStatus = '';
        this.filterServiceStatus = '';
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

    onSelectAll(checked: boolean): void {
        if (checked) {
            this.resources.forEach(res => this.selectedIps.add(res.private_ip));
        } else {
            this.selectedIps.clear();
        }
    }

    onSelectRow(ip: string, checked: boolean): void {
        if (checked) {
            this.selectedIps.add(ip);
        } else {
            this.selectedIps.delete(ip);
        }
    }

    onConfigCenter(): void {
        this.systemService.getConfig('asset_sync_webhook_url').subscribe({
            next: (res) => {
                this.webhookUrl = res?.configValue || '';
                this.dialogService.open(this.configDialog);
            },
            error: () => {
                this.webhookUrl = '';
                this.dialogService.open(this.configDialog);
            }
        });
    }

    saveConfig(ref: any): void {
        this.systemService.updateConfig('asset_sync_webhook_url', this.webhookUrl).subscribe({
            next: () => {
                this.toastrService.success('配置已保存', '成功');
                ref.close();
            },
            error: (err) => {
                this.toastrService.danger(err.message || '保存失败', '出错了');
            }
        });
    }

    onAdd(): void {
        this.dialogService.open(AssetEditDialogComponent, {
            context: {
                mode: 'add',
                filterOptions: this.filterOptions,
            },
        }).onClose.subscribe(data => {
            if (data) {
                const items = Array.isArray(data) ? data : [data];
                this.assetService.importResources(items).subscribe({
                    next: (res) => {
                        if (res.code === 0) {
                            this.toastrService.success(`成功添加 ${res.data} 条数据`, '成功');
                            this.loadData();
                            this.loadFilterOptions();
                        } else {
                            this.toastrService.danger(res.message, '添加失败');
                        }
                    },
                    error: (err) => {
                        this.toastrService.danger(err.message, '添加失败');
                    },
                });
            }
        });
    }

    onEdit(row: ResourceAsset): void {
        this.dialogService.open(AssetEditDialogComponent, {
            context: {
                mode: 'edit',
                resource: row,
                filterOptions: this.filterOptions,
            },
        }).onClose.subscribe(updatedData => {
            if (updatedData) {
                this.assetService.updateResource(updatedData).subscribe({
                    next: (res) => {
                        if (res.code === 0) {
                            this.toastrService.success('更新成功', '成功');
                            this.loadData();
                        } else {
                            this.toastrService.danger(res.message, '更新失败');
                        }
                    },
                    error: (err) => {
                        this.toastrService.danger(err.message, '更新失败');
                    },
                });
            }
        });
    }

    onDelete(row: ResourceAsset): void {
        const itemName = row.private_ip;
        this.confirmDialogService.confirmDelete(itemName).subscribe(confirmed => {
            if (confirmed) {
                this.assetService.deleteResources([row.private_ip]).subscribe({
                    next: (res) => {
                        if (res.code === 0) {
                            this.toastrService.success('删除成功', '成功');
                            this.loadData();
                            this.loadFilterOptions();
                        } else {
                            this.toastrService.danger(res.message, '删除失败');
                        }
                    },
                    error: (err) => {
                        this.toastrService.danger(err.message, '删除失败');
                    },
                });
            }
        });
    }

    onDeleteSelected(): void {
        const ips = Array.from(this.selectedIps);
        if (ips.length === 0) return;

        this.confirmDialogService.confirm(`确定要删除选中的 ${ips.length} 条记录吗？`, '批量删除确认')
            .subscribe(confirmed => {
                if (confirmed) {
                    this.assetService.deleteResources(ips).subscribe({
                        next: (res) => {
                            if (res.code === 0) {
                                this.toastrService.success('删除成功', '成功');
                                this.loadData();
                            } else {
                                this.toastrService.danger(res.message, '删除失败');
                            }
                        },
                        error: (err) => {
                            this.toastrService.danger(err.message, '删除失败');
                        }
                    });
                }
            });
    }

    onExportSelected(): void {
        if (this.selectedIps.size === 0) {
            this.toastrService.warning('请选择要导出的数据', '提示');
            return;
        }

        const dataToExport = this.resources
            .filter(res => this.selectedIps.has(res.private_ip))
            .map(res => {
                const row: any = {};
                this.allColumns.forEach(col => {
                    row[col.label] = res[col.key];
                });
                return row;
            });

        const ws: XLSX.WorkSheet = XLSX.utils.json_to_sheet(dataToExport);
        const wb: XLSX.WorkBook = XLSX.utils.book_new();
        XLSX.utils.book_append_sheet(wb, ws, 'Resources');
        XLSX.writeFile(wb, `asset_export_${new Date().getTime()}.xlsx`);
    }

    onImportClick(fileInput: HTMLInputElement): void {
        fileInput.click();
    }

    onFileChange(event: any): void {
        const target: DataTransfer = <DataTransfer>(event.target);
        if (target.files.length !== 1) return;

        const reader: FileReader = new FileReader();
        reader.onload = (e: any) => {
            const bstr: string = e.target.result;
            const wb: XLSX.WorkBook = XLSX.read(bstr, { type: 'binary' });
            const wsname: string = wb.SheetNames[0];
            const ws: XLSX.WorkSheet = wb.Sheets[wsname];
            const data = XLSX.utils.sheet_to_json(ws);
            this.processImportData(data);
        };
        reader.readAsBinaryString(target.files[0]);
        // Reset file input
        event.target.value = '';
    }

    processImportData(data: any[]): void {
        if (data.length === 0) {
            this.toastrService.warning('Excel文件为空', '提示');
            return;
        }

        const mapping: { [key: string]: string } = {
            '实例类型': 'instance_type',
            '实例编号': 'instance_id',
            '实例名称': 'instance_name',
            '项目名称': 'project_name',
            '项目归属': 'project_ownership',
            '服务类型': 'manual_service',
            '国家': 'country',
            '地区': 'region',
            '外网IP': 'public_ip',
            'EIP': 'public_ip',
            '内网IP': 'private_ip',
            '网络标识': 'network_identifier',
            'CPU': 'cpu',
            '内存': 'memory',
            '存储': 'storage',
            '发行版本': 'release',
            '创建时间': 'created_at',
            '备注': 'remark',
        };

        const items = data.map(row => {
            const item: any = {};
            Object.keys(row).forEach(key => {
                const mappedKey = mapping[key] || key;
                let value = row[key];
                // Convert numbers to strings for fields that backend expects as strings
                if (typeof value === 'number') {
                    value = value.toString();
                }
                item[mappedKey] = value;
            });
            return item;
        });

        // Validation: Instance Type and Private IP are required
        const invalidItems = items.filter(item => !item.instance_type || !item.private_ip);
        if (invalidItems.length > 0) {
            this.toastrService.danger('存在不规范数据（实例类型、内网IP不能为空）', '校验错误');
            return;
        }

        this.isImporting = true;
        this.assetService.importResources(items).subscribe({
            next: (res) => {
                if (res.code === 0) {
                    this.toastrService.success(`成功导入 ${res.data} 条数据`, '导入成功');
                    this.loadData();
                } else {
                    this.toastrService.danger(res.message, '导入失败');
                }
                this.isImporting = false;
            },
            error: (err) => {
                this.toastrService.danger(err.message, '导入失败');
                this.isImporting = false;
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
        if (totalPages <= 7) return Array.from({ length: totalPages }, (_, i) => i + 1);

        const pages: (number | string)[] = [1];
        if (this.page > 4) pages.push('...');

        let start = Math.max(2, this.page - 2);
        let end = Math.min(totalPages - 1, this.page + 2);

        if (this.page <= 4) {
            end = 5;
            start = 2;
        } else if (this.page >= totalPages - 3) {
            start = totalPages - 4;
            end = totalPages - 1;
        }

        for (let i = start; i <= end; i++) pages.push(i);
        if (this.page < totalPages - 3) pages.push('...');
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

    getServiceTags(serviceType: string): string[] {
        if (!serviceType) return [];
        return serviceType.split(',').map(s => s.trim()).filter(s => s !== '');
    }

    getAllServices(res: ResourceAsset): any[] {
        const manual = this.getServiceTags(res.manual_service).map(name => ({ name, type: 'manual' }));
        const auto = (res.auto_services || []).map((s: any) => ({
            name: s.name,
            type: 'auto',
            state: s.state
        }));
        return [...manual, ...auto];
    }

    formatMemory(memory: any): string {
        if (!memory) return '';
        const bytes = parseInt(memory, 10);
        if (isNaN(bytes)) return memory;
        const gb = bytes / (1024 * 1024 * 1024);
        return Math.round(gb) + 'G';
    }

    onApply(dialog: TemplateRef<any>): void {
        this.applyForm.ipList = Array.from(this.selectedIps).join('\n');
        this.dialogService.open(dialog);
    }

    submitApply(ref: any): void {
        const ips = this.applyForm.ipList.split('\n')
            .map(ip => ip.trim())
            .filter(ip => ip !== '');

        if (ips.length === 0) {
            this.toastrService.warning('请输入至少一个 IP 地址', '参数错误');
            return;
        }

        if (!this.applyForm.cookie) {
            this.toastrService.warning('请输入认证 Cookie', '参数错误');
            return;
        }

        this.isApplying = true;
        this.assetService.applyResources({
            ip_list: ips,
            cookie: this.applyForm.cookie,
            remarks: this.applyForm.remarks,
        }).subscribe({
            next: (res) => {
                this.isApplying = false;
                if (res.code === 0) {
                    ref.close();
                    this.dialogService.open(this.applyResultDialog, { context: res.data });
                } else {
                    this.toastrService.danger(res.message, '申请异常');
                }
            },
            error: (err) => {
                this.isApplying = false;
                this.toastrService.danger(err.message || '网络请求故障', '申请失败');
            }
        });
    }

    onServiceClick(serviceName: string, ip: string): void {
        this.currentService = serviceName;
        this.currentIp = ip;
        this.serviceLogContent = '';
        this.dialogService.open(this.serviceLogDialog);
        this.onServiceOp('服务状态');
    }

    onServiceOp(opType: string): void {
        this.isOpLoading = true;
        this.serviceLogContent = opType === '服务状态' ? '正在获取服务状态...' : `正在执行 ${opType}...`;

        this.assetService.serviceOperation({
            type: opType,
            service: this.currentService,
            ip: this.currentIp
        }).subscribe({
            next: (res) => {
                this.isOpLoading = false;
                if (res.code === 0) {
                    this.serviceLogContent = res.data;
                } else {
                    this.toastrService.danger(res.message, `${opType}失败`);
                    this.serviceLogContent = `错误: ${res.message}`;
                }
            },
            error: (err) => {
                this.isOpLoading = false;
                this.toastrService.danger(err.message || '网络请求故障', '操作失败');
                this.serviceLogContent = `请求出错: ${err.message}`;
            }
        });
    }
}

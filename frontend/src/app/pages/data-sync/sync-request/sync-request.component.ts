import { Component, OnInit, TemplateRef, ViewChild } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import { NbDialogService, NbToastrService } from '@nebular/theme';
import { AuthService } from '../../../@core/data/auth.service';
import { SystemService } from '../../../@core/services/system.service';

interface TableItem {
    name: string;
    selected: boolean;
    dbName: string;
}

interface DatabaseItem {
    name: string;
    expanded: boolean;
    tables: TableItem[];
    allSelected?: boolean;
    loading?: boolean;
}

@Component({
    selector: 'ngx-sync-request',
    templateUrl: './sync-request.component.html',
    styleUrls: ['./sync-request.component.scss'],
})
export class SyncRequestComponent implements OnInit {
    @ViewChild('configDialog') configDialog: TemplateRef<any>;

    isAdmin = false;
    activeTab = 'config'; // 'config' or 'records'
    webhookUrl = '';
    country = '';
    countries = ['巴基斯坦', '印尼', '菲律宾', '泰国', '中国', '墨西哥'];
    remark = '';

    // Source DB
    sourceConn = {
        ip: '',
        port: '3306',
    };
    sourceConnected = false;
    sourceLoading = false;
    sourceDatabases: DatabaseItem[] = [];
    sourceError = '';

    // Destination DB
    destConn = {
        ip: '',
        port: '3306',
    };
    destConnected = false;
    destLoading = false;
    destError = '';

    // Selected Tables for Migration
    selectedTables: TableItem[] = [];

    // Sync Records List
    syncRecords: any[] = [];
    recordsLoading = false;
    dutyPersonnel: any[] = [];
    selectedRecord: any = null;
    approvalTables: any[] = [];
    currentApprovalTableIndex = 0;
    validationLoading = false;

    constructor(
        private http: HttpClient,
        private toastr: NbToastrService,
        private dialogService: NbDialogService,
        private authService: AuthService,
        private systemService: SystemService
    ) { }

    ngOnInit(): void {
        console.log('SYNC_REQUEST_V5');
        this.isAdmin = this.authService.canApproveDataSync();
        this.loadConfig();
        this.loadSyncRecords();
        this.loadDutyPersonnel();
    }

    loadDutyPersonnel() {
        this.http.get('/api/duty/personnel').subscribe({
            next: (res: any) => {
                this.dutyPersonnel = res || [];
            }
        });
    }

    onProcessorChange(row: any) {
        this.http.put(`/api/data-sync/list/${row.id}/processor`, {
            processor: row.processor
        }).subscribe({
            next: (res: any) => {
                if (res && res.code === 0) {
                    this.toastr.success('处理人更新成功', '成功');
                } else {
                    this.toastr.danger('处理人更新失败: ' + res.message, '错误');
                }
            },
            error: (err) => {
                this.toastr.danger('更新失败: ' + (err.error?.message || err.message), '错误');
            }
        });
    }

    onStatusChange(row: any) {
        this.http.put(`/api/data-sync/list/${row.id}/status`, {
            approval_status: row.approval_status
        }).subscribe({
            next: (res: any) => {
                if (res && res.code === 0) {
                    this.toastr.success('状态更新成功', '成功');
                } else {
                    this.toastr.danger('状态更新失败: ' + res.message, '错误');
                }
            },
            error: (err) => {
                this.toastr.danger('更新失败: ' + (err.error?.message || err.message), '错误');
            }
        });
    }

    goToApproval(row: any) {
        this.selectedRecord = row;
        this.activeTab = 'approval';
        try {
            const tables = JSON.parse(row.selected_tables);
            this.approvalTables = tables.map((t: any) => ({
                ...t,
                selected: true,
                sql: '',
                schemaLoading: false
            }));
            this.currentApprovalTableIndex = 0;
        } catch (e) {
            this.approvalTables = [];
        }

        // Auto switch status to "处理中" if it's "待审批"
        if (row.approval_status === '待审批') {
            row.approval_status = '处理中';
            this.onStatusChange(row);
        }
    }

    selectApprovalTable(index: number) {
        this.currentApprovalTableIndex = index;
    }

    validateData() {
        if (!this.selectedRecord || this.approvalTables.length === 0) return;

        const selectedToValidate = this.approvalTables.filter(t => t.selected);
        if (selectedToValidate.length === 0) {
            this.toastr.warning('请至少选择一个需要校验的表', '提示');
            return;
        }

        this.validationLoading = true;
        const promises = selectedToValidate.map(table => {
            table.validationStatus = 'loading';
            const payload = {
                country: this.selectedRecord.country,
                ip: this.selectedRecord.source_ip,
                port: this.selectedRecord.source_port,
                db: table.dbName,
                command: `SELECT COUNT(*) FROM ${table.dbName}.${table.name};`,
                type: '校验',
            };

            return this.http.post('/api/data-sync/proxy-webhook', payload).toPromise()
                .then((res: any) => {
                    const data = Array.isArray(res) ? res[0] : res;
                    if (data && data.stdout) {
                        // Extract number from stdout. Assuming format: "COUNT(*)\n123456" or similar
                        const lines = data.stdout.split('\n').map(l => l.trim()).filter(l => l);
                        // Usually the last line or the line after header is the count
                        const countStr = lines.find(l => /^\d+$/.test(l));
                        if (countStr) {
                            const count = parseInt(countStr, 10);
                            table.validationResult = {
                                count: count,
                                passed: count < 200000000
                            };
                            table.validationStatus = 'success';
                        } else {
                            table.validationStatus = 'failed';
                            table.validationError = '无法解析计数结果';
                        }
                    } else {
                        table.validationStatus = 'failed';
                        table.validationError = '响应数据为空或格式错误';
                    }
                })
                .catch(err => {
                    table.validationStatus = 'failed';
                    table.validationError = err.message || '请求失败';
                });
        });

        Promise.all(promises).finally(() => {
            this.validationLoading = false;
            this.toastr.info('数据校验完成', '提示');
        });
    }

    allValidated(): boolean {
        const selected = this.approvalTables.filter(t => t.selected);
        if (selected.length === 0) return false;
        return selected.every(t => t.validationStatus === 'success' && t.validationResult?.passed);
    }

    hasPassedTables(): boolean {
        return this.approvalTables.some(t => t.selected && t.validationStatus === 'success' && t.validationResult?.passed);
    }

    generateSchemas() {
        // Only keep tables that passed validation
        this.approvalTables.forEach(t => {
            if (t.selected && !(t.validationStatus === 'success' && t.validationResult?.passed)) {
                t.selected = false;
            }
        });

        const selected = this.approvalTables.filter(t => t.selected);
        if (selected.length === 0) return;

        // Reset current index to first selected table
        this.currentApprovalTableIndex = this.approvalTables.findIndex(t => t.selected);

        selected.forEach((table, index) => {
            table.schemaLoading = true;
            table.sql = '正在获取表结构...';
            const payload = {
                country: this.selectedRecord.country,
                ip: this.selectedRecord.source_ip,
                port: this.selectedRecord.source_port,
                db: table.dbName,
                command: `show create table ${table.dbName}.${table.name};`,
                type: 'schema',
            };

            this.http.post('/api/data-sync/proxy-webhook', payload).subscribe({
                next: (res: any) => {
                    const data = Array.isArray(res) ? res[0] : res;
                    if (data && data.stdout) {
                        const raw = data.stdout;
                        // 1. Find the start of the actual SQL statement
                        const createIdx = raw.toUpperCase().indexOf('CREATE TABLE');
                        if (createIdx !== -1) {
                            let sql = raw.substring(createIdx);
                            // 2. Replace literal \n with actual newlines
                            sql = sql.replace(/\\n/g, '\n');
                            // 3. Replace literal \" with actual quotes if they exist (standard JSON escaping issues)
                            sql = sql.replace(/\\"/g, '"');
                            table.sql = sql.trim();
                        } else {
                            table.sql = raw.trim();
                        }
                    } else {
                        table.sql = '-- 获取表结构失败：响应格式错误';
                    }
                    table.schemaLoading = false;
                },
                error: (err) => {
                    table.sql = '-- 获取表结构失败：' + (err.message || '网络错误');
                    table.schemaLoading = false;
                }
            });
        });
    }

    approveSync() {
        if (!this.selectedRecord) return;
        this.http.put(`/api/data-sync/list/${this.selectedRecord.id}/approve`, {}).subscribe({
            next: (res: any) => {
                if (res && res.code === 0) {
                    this.toastr.success('审批成功！', '成功');
                    this.selectedRecord.approval_status = '已完成';
                    this.selectedRecord.finished_at = new Date(); // Local update for UI
                    this.selectTab('records');
                } else {
                    this.toastr.danger('审批失败: ' + res.message, '错误');
                }
            },
            error: (err) => {
                this.toastr.danger('审批请求失败: ' + (err.error?.message || err.message), '错误');
            }
        });
    }

    selectTab(tab: string) {
        this.activeTab = tab;
        if (tab !== 'approval') {
            this.selectedRecord = null;
        }
        if (tab === 'records') {
            this.loadSyncRecords();
        }
    }

    loadConfig() {
        this.systemService.getConfig('data_sync_webhook_url').subscribe({
            next: (res) => {
                this.webhookUrl = res?.configValue || 'https://example.com/webhook/d6db9559-b0ce-42db-b472-3b803579cf19';
            },
            error: () => {
                this.webhookUrl = 'https://example.com/webhook/d6db9559-b0ce-42db-b472-3b803579cf19';
            }
        });
    }

    openConfig() {
        this.dialogService.open(this.configDialog);
    }

    saveConfig(ref: any) {
        this.systemService.updateConfig('data_sync_webhook_url', this.webhookUrl).subscribe({
            next: () => {
                this.toastr.success('配置已保存', '成功');
                ref.close();
            },
            error: (err) => {
                this.toastr.danger(err.message || '保存失败', '错误');
            }
        });
    }

    connectSource() {
        if (!this.country) {
            this.toastr.warning('请选择归属国家 (Region)', '提示');
            return;
        }
        if (!this.sourceConn.ip) {
            this.toastr.warning('请输入源库 IP (Host)', '提示');
            return;
        }

        this.sourceLoading = true;
        this.sourceError = '';
        const payload = {
            country: this.country,
            ip: this.sourceConn.ip,
            port: this.sourceConn.port,
            command: 'show databases',
            type: '连接',
        };

        this.http.post('/api/data-sync/proxy-webhook', payload).subscribe({
            next: (res: any) => {
                if (res && res.stdout) {
                    const lines = res.stdout.split('\n')
                        .map(l => l.trim())
                        .filter(l => l && l !== 'Database');

                    if (lines.length > 0) {
                        this.toastr.success('源库连接成功', '成功');
                        this.sourceConnected = true;
                        this.sourceDatabases = lines.map(dbName => ({
                            name: dbName,
                            expanded: false,
                            tables: [],
                            allSelected: false
                        }));
                    } else {
                        this.toastr.danger('连接未返回任何数据库', '错误');
                        this.sourceConnected = false;
                        this.sourceDatabases = [];
                    }
                } else {
                    this.toastr.danger('源库连接失败: 响应数据空', '错误');
                    this.sourceConnected = false;
                    this.sourceDatabases = [];
                }
                this.sourceLoading = false;
            },
            error: (err) => {
                console.error('Source connect error:', err);
                let message = '';
                if (err.error) {
                    if (typeof err.error === 'object') {
                        message = err.error.message || JSON.stringify(err.error);
                    } else if (typeof err.error === 'string') {
                        try {
                            const parsed = JSON.parse(err.error);
                            message = parsed.message || err.error;
                        } catch {
                            message = err.error;
                        }
                    }
                }

                if (!message) {
                    message = err.message || '源库缺失';
                }

                this.sourceError = '';
                this.toastr.danger('请联系管理员添加数据库：' + message, '错误');
                this.sourceLoading = false;
                this.sourceConnected = false;
                this.sourceDatabases = [];
            }
        });
    }

    connectDest() {
        if (!this.country) {
            this.toastr.warning('请选择归属国家 (Region)', '提示');
            return;
        }
        if (!this.destConn.ip) {
            this.toastr.warning('请输入目的库 IP (Host)', '提示');
            return;
        }

        this.destLoading = true;
        this.destError = '';
        const payload = {
            country: this.country,
            ip: this.destConn.ip,
            port: this.destConn.port,
            command: 'show databases',
            type: '连接',
        };

        this.http.post('/api/data-sync/proxy-webhook', payload).subscribe({
            next: (res: any) => {
                if (res && res.stdout) {
                    this.toastr.success('目的库连接成功', '成功');
                    this.destConnected = true;
                } else {
                    this.toastr.danger('目的库连接失败: ' + (res?.stderr || '响应数据空'), '错误');
                    this.destConnected = false;
                }
                this.destLoading = false;
            },
            error: (err) => {
                console.error('Dest connect error:', err);
                let message = '';
                if (err.error) {
                    if (typeof err.error === 'object') {
                        message = err.error.message || JSON.stringify(err.error);
                    } else if (typeof err.error === 'string') {
                        try {
                            const parsed = JSON.parse(err.error);
                            message = parsed.message || err.error;
                        } catch {
                            message = err.error;
                        }
                    }
                }

                if (!message) {
                    message = err.message || '网络错误';
                }

                this.destError = '';
                this.toastr.danger('目的库连接失败网络错误：' + message, '错误');
                this.destLoading = false;
                this.destConnected = false;
            }
        });
    }

    toggleDb(db: DatabaseItem) {
        db.expanded = !db.expanded;
        if (db.expanded && db.tables.length === 0) {
            this.fetchTables(db);
        }
    }

    fetchTables(db: DatabaseItem, autoSelectAll: boolean = false) {
        db.loading = true;
        const payload = {
            country: this.country,
            ip: this.sourceConn.ip,
            port: this.sourceConn.port,
            db: db.name,
            command: `SHOW TABLES FROM ${db.name};`,
            type: '连接',
        };

        const obs = this.http.post('/api/data-sync/proxy-webhook', payload);
        obs.subscribe({
            next: (res: any) => {
                db.loading = false;
                if (res && res.stdout) {
                    const lines = res.stdout.split('\n')
                        .map(l => l.trim())
                        .filter(l => l && !l.startsWith('Tables_in_') && l !== 'Database');

                    db.tables = lines.map(tableName => ({
                        name: tableName,
                        selected: autoSelectAll,
                        dbName: db.name
                    }));
                }
            },
            error: () => {
                db.loading = false;
                this.toastr.danger(`获取表 [${db.name}] 失败`, '错误');
            }
        });
        return obs;
    }

    toggleTableSelection(table: TableItem) {
        table.selected = !table.selected;
    }

    toggleDbSelection(db: DatabaseItem) {
        db.allSelected = !db.allSelected;
        if (db.allSelected && db.tables.length === 0) {
            this.fetchTables(db, true);
        } else {
            db.tables.forEach(t => t.selected = !!db.allSelected);
        }
    }

    async moveToDest() {
        const fetchPromises = [];
        this.sourceDatabases.forEach(db => {
            if (db.allSelected && db.tables.length === 0) {
                fetchPromises.push(this.fetchTables(db, true).toPromise());
            }
        });

        if (fetchPromises.length > 0) {
            try {
                await Promise.all(fetchPromises);
            } catch (e) {
                // Error already handled in fetchTables toast
            }
        }

        this.sourceDatabases.forEach(db => {
            const selectedInDb = db.tables.filter(t => t.selected);
            selectedInDb.forEach(t => {
                if (!this.selectedTables.find(st => st.name === t.name && st.dbName === t.dbName)) {
                    this.selectedTables.push({ ...t, selected: false });
                }
            });
            // Clear selection on left side
            db.tables.forEach(t => t.selected = false);
            db.allSelected = false;
        });
    }

    removeFromDest() {
        this.selectedTables = this.selectedTables.filter(t => !t.selected);
    }

    clearSelectedTables() {
        this.selectedTables = [];
    }

    submitMigration() {
        if (this.selectedTables.length === 0) {
            this.toastr.warning('请至少选择一个表进行迁移', '提示');
            return;
        }

        if (!this.country) {
            this.toastr.warning('请选择库所在国家', '提示');
            return;
        }

        const payload = {
            country: this.country,
            source_ip: this.sourceConn.ip,
            source_port: this.sourceConn.port,
            dest_ip: this.destConn.ip,
            dest_port: this.destConn.port,
            selected_tables: this.selectedTables.map(t => ({ name: t.name, dbName: t.dbName })),
            remark: this.remark,
        };

        this.http.post('/api/data-sync/submit', payload).subscribe({
            next: () => {
                this.toastr.success('工单提交成功！', '成功');
                this.reset();
            },
            error: (err) => {
                this.toastr.danger('工单提交失败: ' + (err.error?.message || err.message), '错误');
            }
        });
    }

    reset() {
        this.selectedTables = [];
        this.sourceConnected = false;
        this.sourceDatabases = [];
        this.destConnected = false;
        this.sourceError = '';
        this.destError = '';
        this.remark = '';
        this.country = '';
        // Optionally clear connections too
        this.sourceConn.ip = '';
        this.destConn.ip = '';
    }

    loadSyncRecords() {
        this.recordsLoading = true;
        this.http.get('/api/data-sync/list').subscribe({
            next: (res: any) => {
                if (res && res.code === 0) {
                    this.syncRecords = res.data || [];
                }
                this.recordsLoading = false;
            },
            error: (err) => {
                this.toastr.danger('获取申请记录失败: ' + (err.error?.message || err.message), '错误');
                this.recordsLoading = false;
            }
        });
    }

    getSelectedTablesText(jsonStr: string): string {
        try {
            const tables = JSON.parse(jsonStr);
            if (Array.isArray(tables)) {
                return tables.map(t => `${t.dbName}.${t.name}`).join(', ');
            }
            return jsonStr;
        } catch {
            return jsonStr;
        }
    }
}

import { Component, OnInit, OnDestroy, TemplateRef, ViewChild } from '@angular/core';
import { AlertService, AlertHistory } from '../../../@core/services/alert.service';
import { ClusterService, Cluster } from '../../../@core/data/cluster.service';
import { NbToastrService, NbDialogService } from '@nebular/theme';
import { ChatService } from '../../../@core/services/chat.service';
import { SystemService } from '../../../@core/services/system.service';
import { AuthService } from '../../../@core/data/auth.service';

@Component({
  selector: 'ngx-alert-dashboard',
  template: `
    <div class="d-flex justify-content-between align-items-center mb-3">
      <h4 class="mb-0">告警大盘</h4>
      <div *ngIf="isAdmin">
        <button nbButton status="primary" size="medium" (click)="openSettings()" class="settings-main-btn">
          <nb-icon icon="settings-2-outline"></nb-icon>
          配置中心
        </button>
      </div>
    </div>

    <div class="row mb-4">
      <div class="col-md-3" *ngFor="let card of cards">
        <nb-card [status]="card.status" class="summary-card">
          <div class="top-area">
            <div class="title-row">
              <span class="card-title">{{ card.title }}</span>
              <div class="change-info" *ngIf="card.change !== undefined">
                <nb-icon [icon]="card.change > 0 ? 'arrow-up-outline' : 'arrow-down-outline'"></nb-icon>
                <span>{{ card.change > 0 ? '+' : '' }}{{ card.change }}%</span>
              </div>
            </div>
          </div>
          <nb-card-body class="bottom-area">
            <div class="stats-row">
              <div class="stat-item">
                <div class="stat-label">活跃告警</div>
                <div class="stat-value active">{{ card.active }}</div>
              </div>
              <div class="stat-item">
                <div class="stat-label">今日告警</div>
                <div class="stat-value">{{ card.today }}</div>
              </div>
            </div>
            <div class="history-trigger" (click)="showTrend(card)" title="查看历史趋势">
              <nb-icon icon="bar-chart-2-outline"></nb-icon>
            </div>
          </nb-card-body>
        </nb-card>
      </div>
    </div>

    <nb-card class="mt-4">
      <div class="event-tabs">
        <div 
          *ngFor="let tab of eventTabs" 
          class="event-tab-btn" 
          [class.active]="selectedEventTab === tab"
          (click)="onEventTabChange(tab)"
        >
          {{ tab }}
        </div>
      </div>

      <div *ngIf="selectedEventTab === 'SQL 告警事件'">
        <div class="p-3">
          <div class="d-flex justify-content-between align-items-center flex-wrap">
            <div class="d-flex align-items-center flex-wrap">
              <div class="d-flex align-items-center mr-3 mb-1">
                <span class="mr-2 text-hint small">集群:</span>
                <nb-select size="tiny" class="filter-item" [(selected)]="selectedCluster" (selectedChange)="onFilterChange()">
                  <nb-option value="">全部</nb-option>
                  <nb-option *ngFor="let c of historyClusters" [value]="c">{{ c }}</nb-option>
                </nb-select>
              </div>

              <div class="d-flex align-items-center mr-3 mb-1">
                <span class="mr-2 text-hint small">部门:</span>
                <nb-select size="tiny" multiple class="filter-item" placeholder="选择部门" [(selected)]="selectedDepartments" (selectedChange)="onFilterChange()">
                  <nb-option *ngFor="let d of historyDepartments" [value]="d">{{ d }}</nb-option>
                </nb-select>
              </div>

              <div class="d-flex align-items-center mr-3 mb-1">
                <span class="mr-2 text-hint small">状态:</span>
                <nb-select size="tiny" class="filter-item" [(selected)]="selectedStatus" (selectedChange)="onFilterChange()">
                  <nb-option value="">全部</nb-option>
                  <nb-option value="Alerting">告警中</nb-option>
                  <nb-option value="Resolved">已结束</nb-option>
                  <nb-option value="Suppressed">抑制中</nb-option>
                  <nb-option value="Killed">已强杀</nb-option>
                  <nb-option value="Whitelisted">已加白</nb-option>
                </nb-select>
              </div>

              <div class="d-flex align-items-center mr-3 mb-1">
                <span class="mr-2 text-hint small">用户:</span>
                <input nbInput size="tiny" class="filter-item" placeholder="搜索用户" [(ngModel)]="selectedUser" (keyup.enter)="onFilterChange()">
              </div>

              <div class="d-flex align-items-center mr-3 mb-1">
                <span class="mr-2 text-hint small">查询ID:</span>
                <input nbInput size="tiny" class="filter-item" placeholder="搜索查询ID" [(ngModel)]="selectedQueryId" (keyup.enter)="onFilterChange()">
              </div>

              <div class="d-flex align-items-center mr-3 mb-1">
                <span class="mr-2 text-hint small">起止日期:</span>
                <input nbInput size="tiny" type="date" [(ngModel)]="startDate" (change)="onFilterChange()" style="width: 120px; height: 28px; padding: 0 5px;" class="mr-1">
                <input nbInput size="tiny" type="date" [(ngModel)]="endDate" (change)="onFilterChange()" style="width: 120px; height: 28px; padding: 0 5px;">
              </div>

              <button nbButton ghost size="small" (click)="refresh()" title="刷新">
                <nb-icon icon="refresh-outline"></nb-icon>
              </button>
              <button nbButton ghost size="small" (click)="exportCsv()" title="导出 CSV">
                <nb-icon icon="download-outline"></nb-icon>
              </button>
            </div>
          </div>

          <div style="overflow-x: auto; width: 100%; position: relative; margin-top: 1rem;">
            <table class="table table-hover mb-0" style="min-width: 1600px; table-layout: fixed;">
              <thead>
                <tr>
                  <th style="width: 160px; white-space: nowrap;" class="sortable" (click)="onSort('created_at')" [class.active-sort]="sortField === 'created_at'">
                    时间 <nb-icon [icon]="getSortIcon('created_at')"></nb-icon>
                  </th>
                  <th style="width: 120px; white-space: nowrap;">集群</th>
                  <th style="width: 120px; white-space: nowrap;">用户</th>
                  <th style="width: 120px; white-space: nowrap;">部门</th>
                  <th style="width: 140px; white-space: nowrap;">查询 ID</th>
                  <th style="width: 120px; white-space: nowrap;" class="sortable" (click)="onSort('cpu_time')" [class.active-sort]="sortField === 'cpu_time'">
                    CPU 时长 <nb-icon [icon]="getSortIcon('cpu_time')"></nb-icon>
                  </th>
                  <th style="width: 120px; white-space: nowrap;" class="sortable" (click)="onSort('mem_usage')" [class.active-sort]="sortField === 'mem_usage'">
                    内存使用 <nb-icon [icon]="getSortIcon('mem_usage')"></nb-icon>
                  </th>
                  <th style="width: 120px; white-space: nowrap;" class="sortable" (click)="onSort('exec_time')" [class.active-sort]="sortField === 'exec_time'">
                    执行时长 <nb-icon [icon]="getSortIcon('exec_time')"></nb-icon>
                  </th>
                  <th style="width: 120px; white-space: nowrap;" class="sortable" (click)="onSort('scan_rows')" [class.active-sort]="sortField === 'scan_rows'">
                    扫描行数 <nb-icon [icon]="getSortIcon('scan_rows')"></nb-icon>
                  </th>
                  <th style="width: 200px; white-space: nowrap;">告警原因</th>
                  <th style="width: 100px; white-space: nowrap;">状态</th>
                  <th style="width: 150px; white-space: nowrap;">操作</th>
                  <th style="width: 120px; white-space: nowrap;">修复人</th>
                  <th style="width: 180px; white-space: nowrap;">修复情况</th>
                </tr>
              </thead>
              <tbody>
                <tr *ngFor="let h of history">
                  <td style="white-space: nowrap;">{{ h.createdAt | date:'MM-dd HH:mm:ss' }}</td>
                  <td style="white-space: nowrap; max-width: 120px; overflow: hidden; text-overflow: ellipsis;" [title]="h.host">{{ h.host }}</td>
                  <td style="white-space: nowrap; max-width: 120px; overflow: hidden; text-overflow: ellipsis;" [title]="h.user">{{ h.user }}</td>
                  <td style="white-space: nowrap; max-width: 120px; overflow: hidden; text-overflow: ellipsis;" [title]="h.department">{{ h.department || '-' }}</td>
                  <td>
                    <span [title]="h.queryId" style="cursor: help; border-bottom: 1px dotted #ccc;">
                      {{ h.queryId | slice:0:8 }}...
                    </span>
                    <button nbButton ghost size="tiny" status="basic" (click)="copy(h.queryId)" title="复制">
                      <nb-icon icon="copy"></nb-icon>
                    </button>
                  </td>
                  <td style="white-space: nowrap;">{{ h.cpuTime | number:'1.2-2' }}s</td>
                  <td style="white-space: nowrap;">{{ formatBytes(h.memUsage) }}</td>
                  <td style="white-space: nowrap;">{{ h.execTime | number:'1.2-2' }}s</td>
                  <td style="white-space: nowrap;">{{ h.scanRows | number:'1.0-0' || '-' }}</td>
                  <td style="max-width: 150px; white-space: normal;">{{ h.violationDetail }}</td>
                  <td style="white-space: nowrap;">
                    <span class="badge" [ngClass]="getStatusClass(h.status)">
                      {{ getStatusText(h.status) }}
                    </span>
                    <span *ngIf="h.ivrMsgId" class="badge badge-info ml-1" title="IVR Alert Sent">IVR</span>
                  </td>
                  <td style="white-space: nowrap;">
                    <button nbButton ghost size="tiny" status="info" (click)="viewSql(h)" title="查看 SQL">
                      <nb-icon icon="eye"></nb-icon>
                    </button>
                    <button *ngIf="h.status === 'Alerting' || h.status === 'Suppressed'" nbButton ghost size="tiny" status="primary" (click)="whitelistQuery(h)" title="加白">
                      加白
                    </button>
                    <button *ngIf="(h.status === 'Alerting' || h.status === 'Suppressed' || h.status === 'Whitelisted') && h.connectionId !== '0'" nbButton ghost size="tiny" status="danger" (click)="killQuery(h)" title="杀掉查询">
                      KILL
                    </button>
                  </td>
                  <td>
                    <input nbInput size="tiny" fullWidth placeholder="name" [(ngModel)]="h.repairPerson" (blur)="onRepairPersonBlur(h)" 
                      style="height: 24px; padding: 2px 4px; border-radius: 4px;">
                  </td>
                  <td style="max-width: 150px;">
                    <div class="d-flex flex-column align-items-start" style="width: 100%;">
                      <span class="text-truncate d-block" style="font-size: 12px; line-height: 1.2; margin-bottom: 2px; width: 100%;" [title]="h.remark || ''">
                        {{ h.remark || '-' }}
                      </span>
                      <button nbButton ghost size="tiny" status="primary" (click)="addRemark(h)" title="Addtext" style="padding: 0; height: auto;">
                        Addtext
                      </button>
                    </div>
                  </td>
                </tr>
                <tr *ngIf="history.length === 0">
                  <td colspan="12" class="text-center">暂无活动告警</td>
                </tr>
              </tbody>
            </table>
          </div>

          <div *ngIf="totalItems > pageSize" class="mt-3">
            <div class="d-flex justify-content-between align-items-center">
              <div class="text-hint">共 {{ totalItems }} 条记录</div>
              <div class="pagination-container d-flex">
                <button nbButton size="small" ghost [disabled]="page === 1" (click)="changePage(page - 1)">
                  <nb-icon icon="chevron-left-outline"></nb-icon>
                </button>
                <div class="pages d-flex mx-2">
                  <button *ngFor="let p of pages" nbButton size="small" [status]="p === page ? 'primary' : 'basic'" 
                    [appearance]="p === page ? 'filled' : 'ghost'" (click)="changePage(p)" class="mx-1">
                    {{ p }}
                  </button>
                </div>
                <button nbButton size="small" ghost [disabled]="page * pageSize >= totalItems" (click)="changePage(page + 1)">
                  <nb-icon icon="chevron-right-outline"></nb-icon>
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>

      <div *ngIf="selectedEventTab !== 'SQL 告警事件'">
        <div class="p-3">
          <div class="d-flex justify-content-between align-items-center flex-wrap mb-3">
            <div class="d-flex align-items-center flex-wrap">
              <div class="d-flex align-items-center mr-3 mb-1">
                <span class="mr-2 text-hint small">告警名称:</span>
                <input nbInput size="tiny" class="filter-item" placeholder="搜索告警名称" [(ngModel)]="externalRuleNameFilter" (ngModelChange)="onExternalFilterChange()">
              </div>
              <div class="d-flex align-items-center mr-3 mb-1">
                <span class="mr-2 text-hint small">级别:</span>
                <nb-select size="tiny" class="filter-item" [(selected)]="externalSeverityFilter" (selectedChange)="onExternalFilterChange()">
                  <nb-option value="">全部</nb-option>
                  <nb-option value="1">P1</nb-option>
                  <nb-option value="2">P2</nb-option>
                  <nb-option value="3">P3</nb-option>
                </nb-select>
              </div>
              <button nbButton ghost size="small" (click)="fetchExternalEvents(selectedEventTab)" title="刷新" [disabled]="externalLoading">
                <nb-icon icon="refresh-outline" [class.spin]="externalLoading"></nb-icon>
              </button>
            </div>
          </div>

          <div style="overflow-x: auto; width: 100%;" *ngIf="!externalLoading">
            <table class="table table-hover mb-0" style="min-width: 1200px; table-layout: fixed;">
              <thead>
                <tr>
                  <th style="width: 160px;">触发时间</th>
                  <th style="width: 100px;">级别</th>
                  <th style="width: 100px;">状态</th>
                  <th style="width: 200px;">告警名称</th>
                  <th style="width: 120px;">触发值</th>
                  <th style="width: 250px;">标签</th>
                </tr>
              </thead>
              <tbody>
                <tr *ngFor="let e of filteredExternalEvents">
                  <td>{{ e.trigger_time_formatted || e.first_trigger_time_formatted }}</td>
                  <td>
                    <span class="badge" [ngClass]="{
                      'badge-danger': e.severity === 1,
                      'badge-warning': e.severity === 2,
                      'badge-info': e.severity === 3 || !e.severity
                    }">P{{ e.severity || '3' }}</span>
                  </td>
                  <td>
                    <span class="badge" [ngClass]="e.alert_status === 'active' ? 'badge-danger' : 'badge-success'">
                      {{ e.alert_status === 'active' ? '告警中' : '已恢复' }}
                    </span>
                  </td>
                  <td class="text-truncate" [title]="e.rule_name">{{ e.rule_name }}</td>
                  <td class="text-truncate" [title]="e.trigger_value">{{ e.trigger_value }}</td>
                  <td>
                    <div class="d-flex align-items-center" style="width: 100%;">
                      <span class="text-truncate small text-hint mr-2" style="flex: 1;" [title]="e.tags">{{ e.tags }}</span>
                      <button nbButton ghost size="tiny" status="info" (click)="viewExternalTags(e)" title="查看详情" style="padding: 0; min-width: 24px;">
                        <nb-icon icon="eye-outline"></nb-icon>
                      </button>
                    </div>
                  </td>
                </tr>
                <tr *ngIf="filteredExternalEvents.length === 0">
                  <td colspan="6" class="text-center py-4">未找到匹配的告警记录</td>
                </tr>
              </tbody>
            </table>
          </div>
          <div class="p-5 text-center" *ngIf="externalLoading">
            <nb-spinner status="primary"></nb-spinner>
            <p class="mt-2">正在获取数据...</p>
          </div>
        </div>
      </div>
    </nb-card>

    <ng-template #sqlDialog let-data let-ref="dialogRef">
      <nb-card style="width: 800px; max-width: 95vw; max-height: 80vh;">
        <nb-card-header>SQL 详情</nb-card-header>
        <nb-card-body>
          <pre style="white-space: pre-wrap; word-wrap: break-word; background: #f5f5f5; padding: 10px; border-radius: 4px; max-height: 55vh; overflow-y: auto;">{{ data.sqlText }}</pre>
        </nb-card-body>
        <nb-card-footer class="d-flex justify-content-between">
          <div class="d-flex gap-2">
            <button *ngIf="(data.status === 'Alerting' || data.status === 'Suppressed' || data.status === 'Whitelisted') && data.connectionId !== '0'" nbButton status="danger" size="small" (click)="killFromDetail(data.id, ref)">KILL</button>
            <button *ngIf="data.status === 'Alerting' || data.status === 'Suppressed'" nbButton status="primary" size="small" (click)="whitelistFromDetail(data.id, ref)">加白</button>
            <button nbButton status="success" size="small" (click)="optimizeSqlViaAssistant(data, ref)">优化</button>
          </div>
          <button nbButton status="basic" size="small" (click)="ref.close()">关闭</button>
        </nb-card-footer>
      </nb-card>
    </ng-template>

    <ng-template #trendDialog let-data let-ref="dialogRef">
      <nb-card style="width: 800px; max-width: 95vw;">
        <nb-card-header class="d-flex justify-content-between align-items-center">
          <div class="d-flex align-items-center">
            <span class="mr-3">{{ data.title }} 历史趋势</span>
            <nb-select size="tiny" [(selected)]="selectedTrendDays" (selectedChange)="onTrendDaysChange(data)">
              <nb-option [value]="7">过去 7 天</nb-option>
              <nb-option [value]="14">过去 14 天</nb-option>
              <nb-option [value]="30">过去 30 天</nb-option>
              <nb-option [value]="180">过去 180 天</nb-option>
              <nb-option [value]="360">过去 360 天</nb-option>
            </nb-select>
          </div>
          <button nbButton ghost size="tiny" (click)="ref.close()"><nb-icon icon="close-outline"></nb-icon></button>
        </nb-card-header>
        <nb-card-body>
          <div *ngIf="trendLoading" class="text-center p-5">
            <nb-icon icon="loader-outline" [options]="{ animation: { type: 'pulse' } }"></nb-icon>
            <div class="text-hint mt-2">正在加载趋势数据...</div>
          </div>
          <div *ngIf="!trendLoading && trendOptions && trendOptions.series" echarts [options]="trendOptions" style="height: 400px; width: 100%;"></div>
          <div *ngIf="!trendLoading && (!trendOptions || !trendOptions.series)" class="text-center p-5 text-hint">暂无趋势数据</div>
        </nb-card-body>
      </nb-card>
    </ng-template>

    <ng-template #settingsDialog let-ref="dialogRef">
      <nb-card style="width: 500px;">
        <nb-card-header>仪表盘配置</nb-card-header>
        <nb-card-body>
          <div class="form-group">
            <label class="label">外部告警 Webhook 地址</label>
            <input nbInput fullWidth [(ngModel)]="webhookUrl" placeholder="https://...">
            <p class="text-hint small mt-2">主机、组件、数据告警板块的数据来源地址</p>
          </div>
        </nb-card-body>
        <nb-card-footer class="d-flex justify-content-end">
          <button nbButton ghost status="basic" (click)="ref.close()" class="mr-2">取消</button>
          <button nbButton status="primary" (click)="saveSettings(ref)">保存</button>
        </nb-card-footer>
      </nb-card>
    </ng-template>

    <ng-template #remarkDialog let-ref="dialogRef">
      <nb-card style="min-width: 400px;">
        <nb-card-header>修复情况 (Repair Progress)</nb-card-header>
        <nb-card-body>
          <textarea nbInput fullWidth placeholder="请输入修复情况..." [(ngModel)]="remarkValue" rows="5"></textarea>
        </nb-card-body>
        <nb-card-footer class="d-flex justify-content-end">
          <button nbButton ghost status="basic" (click)="ref.close()" class="mr-2">取消</button>
          <button nbButton status="primary" (click)="saveRemark(ref)">保存</button>
        </nb-card-footer>
      </nb-card>
    </ng-template>

    <ng-template #confirmKillDialog let-ref="dialogRef">
      <nb-card style="min-width: 300px;">
        <nb-card-header>确认操作</nb-card-header>
        <nb-card-body>
          确定要杀掉该查询吗？
        </nb-card-body>
        <nb-card-footer class="d-flex justify-content-end">
          <button nbButton ghost status="basic" (click)="ref.close(false)" class="mr-2">取消</button>
          <button nbButton status="danger" (click)="ref.close(true)">确定杀掉</button>
        </nb-card-footer>
      </nb-card>
    </ng-template>

    <ng-template #confirmWhitelistDialog let-ref="dialogRef">
      <nb-card style="min-width: 300px;">
        <nb-card-header>确认操作</nb-card-header>
        <nb-card-body>
          确定要将此查询加入白名单吗？(不再告警，但继续运行)
        </nb-card-body>
        <nb-card-footer class="d-flex justify-content-end">
          <button nbButton ghost status="basic" (click)="ref.close(false)" class="mr-2">取消</button>
          <button nbButton status="primary" (click)="ref.close(true)">确定加白</button>
        </nb-card-footer>
      </nb-card>
    </ng-template>

    <ng-template #tagsDialog let-data let-ref="dialogRef">
      <nb-card style="width: 600px; max-width: 95vw;">
        <nb-card-header>标签详情 (Tags Detail)</nb-card-header>
        <nb-card-body>
          <div style="white-space: pre-wrap; word-wrap: break-word; background: #f5f5f5; padding: 15px; border-radius: 4px; font-family: monospace; font-size: 13px; color: #333; line-height: 1.5; border: 1px solid #e4e9f2;">{{ data.tags }}</div>
        </nb-card-body>
        <nb-card-footer class="d-flex justify-content-end">
          <button nbButton status="basic" size="small" (click)="ref.close()">关闭</button>
        </nb-card-footer>
      </nb-card>
    </ng-template>
  `,
  styles: [`
    .badge { padding: 2px 8px; border-radius: 4px; color: white; border: none; font-size: 12px; }
    .badge-danger { background-color: #ff3d71; }
    .badge-success { background-color: #00d68f; }
    .badge-warning { background-color: #ffaa00; }
    .badge-info { background-color: #3366ff; }
    .badge-secondary { background-color: #8f9bb3; }
    .pages button { min-width: 32px; padding: 0 4px; }
    .sortable { cursor: pointer; user-select: none; white-space: nowrap; transition: color 0.15s ease-in-out; }
    .sortable:hover { background: rgba(0,0,0,0.02); color: #3366ff; }
    .sortable nb-icon { font-size: 14px; vertical-align: middle; margin-left: 2px; }
    .active-sort { color: #3366ff; font-weight: bold; }
    .active-sort nb-icon { color: #3366ff; }
    .text-hint { color: #8f9bb3; }
    .gap-2 { gap: 0.5rem; }
    .filter-item {
      width: 140px !important;
      height: 28px !important;
      max-height: 28px !important;
      font-size: 13px !important;
    }
    :host ::ng-deep .filter-item.nb-select button {
      height: 28px !important;
      padding: 0 0.5rem !important;
      line-height: 1 !important;
      min-height: 28px !important;
    }
    :host ::ng-deep .filter-item.nb-select .select-button {
      min-height: 28px !important;
      padding-top: 0 !important;
      padding-bottom: 0 !important;
    }
    input.filter-item {
      padding: 0 0.5rem !important;
      line-height: 28px !important;
    }

    .summary-card {
      margin-bottom: 0;
      border: none;
      box-shadow: 0 4px 12px rgba(0,0,0,0.08);
      overflow: hidden;
      border-radius: 12px;
    }
    .top-area {
      padding: 1rem 1.25rem;
      color: white;
      transition: all 0.3s;
    }
    .summary-card.status-danger .top-area { background: linear-gradient(135deg, #ff3d71, #ff708d); }
    .summary-card.status-warning .top-area { background: linear-gradient(135deg, #ffaa00, #ffc94d); }
    .summary-card.status-info .top-area { background: linear-gradient(135deg, #3366ff, #6691ff); }
    .summary-card.status-success .top-area { background: linear-gradient(135deg, #00d68f, #2ce69b); }

    .title-row {
      display: flex;
      justify-content: space-between;
      align-items: center;
    }
    .card-title {
      font-size: 1.1rem;
      font-weight: 600;
      opacity: 0.95;
    }
    .change-info {
      display: flex;
      align-items: center;
      font-size: 0.9rem;
      background: rgba(255,255,255,0.2);
      padding: 2px 8px;
      border-radius: 20px;
    }
    .change-info nb-icon {
      font-size: 1rem;
      margin-right: 2px;
    }
    
    .bottom-area {
      background: white;
      padding: 1.25rem;
      position: relative;
    }
    .stats-row {
      display: flex;
      gap: 1.5rem;
    }
    .stat-item {
      display: flex;
      flex-direction: column;
    }
    .stat-label {
      font-size: 0.75rem;
      color: #8f9bb3;
      text-transform: uppercase;
      letter-spacing: 0.5px;
      margin-bottom: 4px;
    }
    .stat-value {
      font-size: 1.25rem;
      font-weight: 700;
      color: #222b45;
    }
    .stat-value.active {
      color: #ff3d71;
    }

    .history-trigger {
      position: absolute;
      right: 1.25rem;
      bottom: 1.25rem;
      cursor: pointer;
      color: #8f9bb3;
      transition: all 0.2s;
      padding: 4px;
      border-radius: 4px;
    }
    .history-trigger:hover {
      color: #3366ff;
      background: #f0f4ff;
    }
    .history-trigger nb-icon {
      font-size: 1.5rem;
    }

    .settings-btn {
      color: #8f9bb3;
      padding: 0 8px;
      transition: all 0.2s;
      display: flex;
      align-items: center;
      height: 32px;
    }
    .settings-btn:hover {
      color: #3366ff;
      background: rgba(51, 102, 255, 0.08);
    }
    .settings-btn nb-icon {
      font-size: 1.1rem;
      margin-right: 4px;
    }
    .settings-main-btn {
      box-shadow: 0 4px 12px rgba(51, 102, 255, 0.25);
      font-weight: 600;
    }

    .event-tabs {
      display: flex;
      gap: 0.75rem;
      padding: 1.25rem 1.25rem 0 1.25rem;
    }
    .event-tab-btn {
      padding: 6px 16px;
      border-radius: 8px;
      font-weight: 600;
      font-size: 0.9rem;
      cursor: pointer;
      border: 1px solid transparent;
      transition: all 0.2s;
      background: #f0f4ff;
      color: #3366ff;
    }
    .event-tab-btn:hover {
      background: #e0e8ff;
    }
    .event-tab-btn.active {
      background: #3366ff;
      color: white;
      box-shadow: 0 4px 12px rgba(51, 102, 255, 0.3);
    }
    .spin {
      animation: spin 1s linear infinite;
    }
    @keyframes spin {
      100% { transform: rotate(360deg); }
    }
    .badge {
      padding: 4px 8px;
      border-radius: 4px;
      font-weight: 600;
      font-size: 0.75rem;
    }
    .badge-danger { background: #ff3d71; color: white; }
    .badge-warning { background: #ffaa00; color: white; }
    .badge-info { background: #0095ff; color: white; }
    .badge-success { background: #00d68f; color: white; }
  `]
})
export class AlertDashboardComponent implements OnInit, OnDestroy {
  cards = [
    { key: 'sql', title: 'SQL 告警', status: 'danger', active: 0, today: 0, change: 0 },
    { key: 'host', title: '基础设施告警', status: 'warning', active: 0, today: 0, change: 0 },
    { key: 'component', title: '服务指标', status: 'info', active: 0, today: 0, change: 0 },
    { key: 'data', title: '数仓任务', status: 'success', active: 0, today: 0, change: 0 },
  ];

  history: AlertHistory[] = [];
  isAdmin = false;
  selectedTrendDays: number = 7;
  webhookUrl = '';
  externalStats: any = {};

  // Pagination & Filter
  page = 1;
  pageSize = 20;
  totalItems = 0;
  selectedStatus = '';
  selectedCluster = '';
  selectedUser = '';
  selectedQueryId = '';
  selectedDepartments: string[] = [];
  sortField = 'created_at';
  sortOrder = 'desc';

  pages: number[] = [];
  historyClusters: string[] = [];
  historyDepartments: string[] = [];

  // Filters
  startDate = '';
  endDate = '';

  // Remark logic
  currentHistory: AlertHistory | null = null;
  remarkValue = '';

  eventTabs = ['SQL 告警事件', '基础设施告警', '服务指标', '数仓任务'];
  selectedEventTab = 'SQL 告警事件';

  // Trend Chart
  trendOptions: any = {};
  trendLoading = false;

  // External Events Table
  externalEvents: any[] = [];
  filteredExternalEvents: any[] = [];
  externalLoading = false;
  externalRuleNameFilter = '';
  externalSeverityFilter = '';

  @ViewChild('sqlDialog') sqlDialog: TemplateRef<any>;
  @ViewChild('confirmKillDialog') confirmKillDialog: TemplateRef<any>;
  @ViewChild('confirmWhitelistDialog') confirmWhitelistDialog: TemplateRef<any>;
  @ViewChild('remarkDialog') remarkDialog: TemplateRef<any>;
  @ViewChild('trendDialog') trendDialog: TemplateRef<any>;
  @ViewChild('settingsDialog') settingsDialog: TemplateRef<any>;
  @ViewChild('tagsDialog') tagsDialog: TemplateRef<any>;

  private refreshTimer: any;

  constructor(
    private alertService: AlertService,
    private clusterService: ClusterService,
    private toastr: NbToastrService,
    private dialogService: NbDialogService,
    private chatService: ChatService,
    private systemService: SystemService,
    private authService: AuthService
  ) { }

  ngOnInit() {
    const end = new Date();
    const start = new Date();
    start.setDate(start.getDate() - 7);

    this.startDate = start.toISOString().split('T')[0];
    this.endDate = end.toISOString().split('T')[0];

    this.authService.currentUser.subscribe(user => {
      this.isAdmin = user?.is_super_admin || user?.is_org_admin || false;
    });

    this.loadMetadata();
    this.refresh(); // refresh() calls loadSummaries()

    this.refreshTimer = setInterval(() => {
      this.refresh();
    }, 10000);
  }

  onEventTabChange(tab: string) {
    this.selectedEventTab = tab;
    if (tab !== 'SQL 告警事件') {
      this.fetchExternalEvents(tab);
    }
  }

  viewExternalTags(e: any) {
    this.dialogService.open(this.tagsDialog, { context: e, closeOnBackdropClick: true });
  }

  fetchExternalEvents(tab: string) {
    this.externalLoading = true;
    this.externalEvents = [];
    this.filteredExternalEvents = [];

    // Map display tab names back to the rule names expected by the external webhook
    let ruleName = tab;
    if (tab === '基础设施告警') ruleName = '主机告警事件';
    else if (tab === '服务指标') ruleName = '组件告警事件';
    else if (tab === '数仓任务') ruleName = '数据告警事件';

    const payload = JSON.stringify({ rule: ruleName });

    this.alertService.getExternalSummary(payload).subscribe({
      next: (res) => {
        try {
          // Parse the stringified sqloutput
          const rawData = typeof res.sqloutput === 'string' ? JSON.parse(res.sqloutput) : (res.sqloutput || []);
          this.externalEvents = Array.isArray(rawData) ? rawData : [];
          this.onExternalFilterChange();
        } catch (e) {
          console.error('Failed to parse external events', e);
          this.toastr.danger('解析外部告警数据失败');
        }
        this.externalLoading = false;
      },
      error: (err) => {
        console.error('Failed to fetch external events', err);
        this.toastr.danger('获取外部告警数据失败');
        this.externalLoading = false;
      }
    });
  }

  onExternalFilterChange() {
    this.filteredExternalEvents = this.externalEvents.filter(e => {
      const matchName = !this.externalRuleNameFilter || (e.rule_name || '').toLowerCase().includes(this.externalRuleNameFilter.toLowerCase());
      const matchSeverity = !this.externalSeverityFilter || e.severity?.toString() === this.externalSeverityFilter;
      return matchName && matchSeverity;
    });
  }

  ngOnDestroy() {
    if (this.refreshTimer) {
      clearInterval(this.refreshTimer);
    }
  }

  loadMetadata() {
    this.alertService.getHistoryClusters().subscribe(c => this.historyClusters = c);
    this.alertService.getHistoryDepartments().subscribe(d => this.historyDepartments = d);
  }

  loadSummaries() {
    // SQL Summary
    this.alertService.getSqlSummary().subscribe(res => {
      const card = this.cards.find(c => c.key === 'sql');
      if (card) {
        card.active = Number(res.activeCount || 0);
        card.today = Number(res.todayCount || 0);
        (card as any).change = Number(res.percentageChange || 0).toFixed(2);
      }
    });

    this.fetchDirectExternalStats();
  }

  private fetchDirectExternalStats() {
    const parse = (val: any) => {
      try {
        const data = typeof val.sqloutput === 'string' ? JSON.parse(val.sqloutput) : (val.sqloutput || []);
        return Array.isArray(data) ? data : [];
      } catch (e) { return []; }
    };

    this.alertService.getExternalSummary(JSON.stringify({ rule: '活跃告警数' })).subscribe(res => {
      const list = parse(res);
      list.forEach((item: any) => {
        const group = item.group_name;
        if (!this.externalStats[group]) this.externalStats[group] = {};
        this.externalStats[group].active_alert = Number(item.active_alert ?? item.activeCount ?? 0);
      });
      this.updateExternalCards();
    });

    this.alertService.getExternalSummary(JSON.stringify({ rule: '今日告警数' })).subscribe(res => {
      const list = parse(res);
      list.forEach((item: any) => {
        const group = item.group_name;
        if (!this.externalStats[group]) this.externalStats[group] = {};
        this.externalStats[group].today_alert = Number(item.today_count ?? item.alert_count ?? 0);
        this.externalStats[group].change_rate = Number(item.change_rate_percentage ?? 0);
      });
      this.updateExternalCards();
    });
  }

  private updateExternalCards() {
    const mappings = [
      { key: 'host', group: '基础设施告警' },
      { key: 'component', group: '服务指标' },
      { key: 'data', group: '数仓任务' },
    ];

    mappings.forEach(m => {
      const stats = this.externalStats[m.group];
      const card = this.cards.find(c => c.key === m.key);
      if (card && stats) {
        // Use ?? to fallback to 0 if field is null/undefined
        card.active = Number(stats.active_alert ?? stats.activeCount ?? 0);
        card.today = Number(stats.today_alert ?? stats.todayCount ?? stats.alert_count ?? 0);
        const rawChange = stats.change_rate ?? stats.percentageChange ?? 0;
        (card as any).change = Number(rawChange).toFixed(2);
      }
    });
  }

  showTrend(card: any) {
    this.selectedTrendDays = 7; // Reset to default when opening
    this.trendLoading = true;
    this.trendOptions = {};

    // Open dialog first
    this.dialogService.open(this.trendDialog, { context: card, closeOnBackdropClick: true });

    // Use setTimeout to ensure dialog container is ready for ECharts
    setTimeout(() => {
      this.loadTrend(card);
    }, 100);
  }

  onTrendDaysChange(card: any) {
    this.trendLoading = true;
    this.loadTrend(card);
  }

  loadTrend(card: any) {
    if (card.key === 'sql') {
      this.alertService.getSqlTrend(this.selectedTrendDays).subscribe({
        next: (res) => {
          if (res && Array.isArray(res) && res.length > 0) {
            this.renderTrendChart(res);
          } else {
            this.trendOptions = {};
          }
          this.trendLoading = false;
        },
        error: (err) => {
          console.error('SQL trend error:', err);
          this.toastr.danger('获取 SQL 趋势失败');
          this.trendLoading = false;
        }
      });
      return;
    }

    const groupName = card.key === 'host' ? '基础设施告警' : (card.key === 'component' ? '服务指标' : '数仓任务');
    const stats = this.externalStats[groupName];

    // For external alerts, we might have stored more data in trend_data than requested,
    // or we might need to fetch it. For now we use what we have and filter by date.
    if (stats && stats.trend_data && Array.isArray(stats.trend_data) && stats.trend_data.length > 0) {
      this.renderTrendChart(stats.trend_data);
      this.trendLoading = false;
    } else {
      this.alertService.getExternalSummary(JSON.stringify({ rule: "历史趋势", days: this.selectedTrendDays })).subscribe({
        next: (res) => {
          try {
            const data = JSON.parse(res.sqloutput);
            const filtered = data.filter((i: any) => i.group_name === groupName);
            this.renderTrendChart(filtered);
          } catch (e) {
            console.error('Trend parse error:', e);
            this.toastr.danger('解析趋势数据失败');
          }
          this.trendLoading = false;
        },
        error: (err) => {
          this.toastr.danger('获取趋势数据失败');
          this.trendLoading = false;
        }
      });
    }
  }

  renderTrendChart(data: any[]) {
    if (!data || !Array.isArray(data) || data.length === 0) {
      console.log('No data for trend chart');
      this.trendOptions = {};
      return;
    }

    // Sort by date safely and find fields
    const cutoffDate = new Date();
    cutoffDate.setDate(cutoffDate.getDate() - this.selectedTrendDays);
    const cutoffStr = cutoffDate.toISOString().split('T')[0];

    const sortedData = [...data]
      .map(i => {
        // Find alert_count field flexibly
        const count = i.alert_count ?? i.today_count ?? i.count ?? i.alert_num ?? 0;
        // Find alert_date field flexibly
        const date = i.alert_date ?? i.pdate ?? i.dt ?? i.date ?? i.p_date ?? 'Unknown';
        return { ...i, _date: date, _count: Number(count) };
      })
      .filter(i => i._date !== 'Unknown' && i._date >= cutoffStr)
      .sort((a, b) => a._date.localeCompare(b._date));

    this.trendOptions = {
      grid: {
        top: '10%',
        left: '3%',
        right: '4%',
        bottom: this.selectedTrendDays > 30 ? '20%' : '15%',
        containLabel: true
      },
      tooltip: { trigger: 'axis' },
      dataZoom: this.selectedTrendDays > 30 ? [
        { type: 'inside', start: 0, end: 100 },
        { type: 'slider', height: 20, bottom: 10 }
      ] : [],
      xAxis: {
        type: 'category',
        data: sortedData.map(i => i._date),
        axisLabel: {
          rotate: this.selectedTrendDays > 14 ? 45 : 0,
          interval: 'auto'
        }
      },
      yAxis: { type: 'value' },
      series: [{
        name: '告警数',
        data: sortedData.map(i => i._count),
        type: 'line',
        smooth: true,
        areaStyle: { opacity: 0.1 },
        itemStyle: { color: '#3366ff' }
      }]
    };
  }

  openSettings() {
    this.systemService.getConfig('alert_webhook_url').subscribe(res => {
      this.webhookUrl = res?.configValue || '';
      this.dialogService.open(this.settingsDialog);
    });
  }

  saveSettings(ref: any) {
    this.systemService.updateConfig('alert_webhook_url', this.webhookUrl).subscribe({
      next: () => {
        this.toastr.success('配置已保存', '成功');
        ref.close();
        this.loadSummaries();
      },
      error: (err) => this.toastr.danger('保存失败: ' + err.message)
    });
  }

  refresh() {
    this.alertService.getHistory(
      this.page,
      this.pageSize,
      this.selectedStatus,
      this.selectedCluster,
      this.selectedUser,
      this.selectedDepartments.join(','),
      this.sortField,
      this.sortOrder,
      this.startDate,
      this.endDate,
      this.selectedQueryId
    ).subscribe(res => {
      this.history = res.items;
      this.totalItems = res.total;
      this.updatePages();
    });
    this.loadSummaries();
  }

  // --- Utility methods kept from original ---
  exportCsv() {
    this.alertService.getHistory(1, 10000, this.selectedStatus, this.selectedCluster, this.selectedUser, this.selectedDepartments.join(','), this.sortField, this.sortOrder, this.startDate, this.endDate, this.selectedQueryId).subscribe(res => {
      const items = res.items;
      if (items.length === 0) return;
      const headers = ['时间', '集群', '用户', '部门', '查询ID', 'CPU时长(s)', '内存使用', '执行时长(s)', '扫描行数', '告警原因', '状态', '修复人', '修复情况'];
      const rows = items.map(h => [h.createdAt, h.host, h.user, h.department || '-', h.queryId, h.cpuTime, this.formatBytes(h.memUsage), h.execTime, h.scanRows || 0, h.violationDetail, this.getStatusText(h.status || ''), h.repairPerson || '', h.remark || '']);
      let csvContent = "\ufeff" + headers.join(",") + "\n";
      rows.forEach(row => { csvContent += row.map(cell => `"${(cell || '').toString().replace(/"/g, '""')}"`).join(",") + "\n"; });
      const blob = new Blob([csvContent], { type: 'text/csv;charset=utf-8;' });
      const link = document.createElement("a");
      link.href = URL.createObjectURL(blob);
      link.download = `alert_history_${Date.now()}.csv`;
      link.click();
    });
  }

  onFilterChange() { this.page = 1; this.refresh(); }
  onSort(field: string) { if (this.sortField === field) { this.sortOrder = this.sortOrder === 'asc' ? 'desc' : 'asc'; } else { this.sortField = field; this.sortOrder = 'desc'; } this.refresh(); }
  getSortIcon(field: string): string { if (this.sortField !== field) return 'hash-outline'; return this.sortOrder === 'asc' ? 'arrow-up-outline' : 'arrow-down-outline'; }
  changePage(p: number) { if (p < 1 || p > Math.ceil(this.totalItems / this.pageSize)) return; this.page = p; this.refresh(); }
  updatePages() { const totalPages = Math.ceil(this.totalItems / this.pageSize); this.pages = []; let start = Math.max(1, this.page - 3); let end = Math.min(totalPages, start + 6); for (let i = start; i <= end; i++) this.pages.push(i); }

  viewSql(history: AlertHistory) {
    if (!history.sqlText || history.sqlText === 'No SQL content' || history.sqlText.includes('...')) {
      this.alertService.getHistoryDetail(history.id).subscribe(detail => {
        history.sqlText = detail.sqlText;
        this.dialogService.open(this.sqlDialog, { context: history });
      });
    } else {
      this.dialogService.open(this.sqlDialog, { context: history });
    }
  }

  optimizeSqlViaAssistant(h: AlertHistory, ref: any) {
    const prompt = `请分析并优化以下 StarRocks 查询：\nSQL 内容:\n${h.sqlText}`;
    this.chatService.triggerModule('sql优化助手', { sql: prompt }, prompt);
    ref.close();
  }

  whitelistQuery(history: AlertHistory) {
    this.dialogService.open(this.confirmWhitelistDialog).onClose.subscribe(confirmed => {
      if (confirmed) this.alertService.whitelistQuery(history.id).subscribe(() => this.refresh());
    });
  }

  whitelistFromDetail(id: number, ref: any) {
    this.alertService.whitelistQuery(id).subscribe(() => { ref.close(); this.refresh(); });
  }

  killFromDetail(id: number, ref: any) {
    this.alertService.killQuery(id).subscribe(() => { ref.close(); this.refresh(); });
  }

  killQuery(history: AlertHistory) {
    this.dialogService.open(this.confirmKillDialog).onClose.subscribe(confirmed => {
      if (confirmed) this.alertService.killQuery(history.id).subscribe(() => this.refresh());
    });
  }

  addRemark(h: AlertHistory) { this.currentHistory = h; this.remarkValue = h.remark || ''; this.dialogService.open(this.remarkDialog); }
  saveRemark(ref: any) {
    if (!this.currentHistory) return;
    this.alertService.updateRemark(this.currentHistory.id, this.remarkValue).subscribe(() => { ref.close(); this.refresh(); });
  }

  onRepairPersonBlur(h: AlertHistory) { this.alertService.updateRepairPerson(h.id, h.repairPerson || '').subscribe(); }

  getStatusText(status: string): string {
    switch (status) {
      case 'Alerting': return '告警中';
      case 'Resolved': return '已结束';
      case 'Suppressed': return '抑制中';
      case 'Killed': return '已强杀';
      case 'Whitelisted': return '已加白';
      default: return status || '已结束';
    }
  }

  getStatusClass(status: string): string {
    switch (status) {
      case 'Alerting': return 'badge-danger';
      case 'Resolved': return 'badge-success';
      case 'Suppressed': return 'badge-secondary';
      case 'Killed': return 'badge-warning';
      case 'Whitelisted': return 'badge-info';
      default: return 'badge-secondary';
    }
  }

  formatBytes(bytes: number | undefined): string {
    if (!bytes) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
  }

  copy(text: string) { if (navigator.clipboard) navigator.clipboard.writeText(text).then(() => this.toastr.info('已复制到剪贴板')); }
}

import { Component, Input, OnInit } from '@angular/core';
import { NbDialogRef } from '@nebular/theme';
import { AlertRule, AlertReceiver, AlertChannel } from '../../../@core/services/alert.service';
import { ClusterService, Cluster } from '../../../@core/data/cluster.service';
import { ResourceService, ResourceDataSource } from '../../../@core/services/resource.service';
import { HeadcountService, Employee } from '../../../@core/services/headcount.service';
import { map } from 'rxjs/operators';

@Component({
  selector: 'ngx-alert-rule-detail',
  template: `
    <nb-card class="rule-detail-card">
      <nb-card-header>{{ rule.id ? '编辑规则' : '新增规则' }}</nb-card-header>
      <nb-card-body cdkScrollable>
        <!-- Basic Info -->
        <div class="row">
          <div class="col-sm-12">
            <div class="form-group">
              <label class="label">规则名称</label>
              <input nbInput fullWidth [(ngModel)]="rule.name" placeholder="请输入规则名称">
            </div>
          </div>
        </div>

        <div class="row">
          <div class="col-sm-4">
            <div class="form-group">
              <label class="label">地区选择</label>
              <nb-select fullWidth [(ngModel)]="rule.region">
                <nb-option value="China">中国 (China)</nb-option>
                <nb-option value="Thailand">泰国 (Thailand)</nb-option>
                <nb-option value="Mexico">墨西哥 (Mexico)</nb-option>
                <nb-option value="Philippines">菲律宾 (Philippines)</nb-option>
                <nb-option value="Pakistan">巴基斯坦 (Pakistan)</nb-option>
                <nb-option value="Indonesia">印尼 (Indonesia)</nb-option>
              </nb-select>
            </div>
          </div>
          <div class="col-sm-4">
            <div class="form-group">
              <label class="label">StarRocks 版本</label>
              <nb-select fullWidth [(ngModel)]="rule.starrocksVersion">
                <nb-option value="3.2">3.2</nb-option>
                <nb-option value="3.3">3.3</nb-option>
              </nb-select>
            </div>
          </div>
          <div class="col-sm-4">
            <div class="form-group">
              <label class="label">数据源 (StarRocks)</label>
              <nb-select fullWidth [(ngModel)]="rule.datasourceId" (selectedChange)="onDataSourceChange($event)">
                <nb-option *ngFor="let ds of dataSources" [value]="ds.id">{{ ds.name }}</nb-option>
              </nb-select>
            </div>
          </div>
        </div>

        <div class="row">
          <div class="col-sm-6">
            <div class="form-group">
              <label class="label">告警子类</label>
              <nb-select fullWidth [(ngModel)]="rule.subType" (selectedChange)="onSubTypeChange()">
                <nb-option value="Memory">内存使用 (Memory)</nb-option>
                <nb-option value="Cpu">CPU 时间 (CPU)</nb-option>
                <nb-option value="ScanRows">扫描行数 (Scan Rows)</nb-option>
                <nb-option value="ExecutionTime">执行时间 (Execution Time)</nb-option>
              </nb-select>
            </div>
          </div>
          <div class="col-sm-6">
            <div class="form-group">
              <label class="label">告警阈值 ({{ getUnit() }})</label>
              <input nbInput fullWidth type="number" [(ngModel)]="rule.threshold" placeholder="请输入阈值">
            </div>
          </div>
        </div>
        <div class="row mt-2 align-items-end">
          <div class="col-sm-3">
            <div class="form-group mb-0 d-flex flex-column justify-content-center">
              <label class="label">自动强杀 (Auto Kill)</label>
              <div style="height: 36px; display: flex; align-items: center;">
                <nb-toggle [(ngModel)]="rule.autoKill" status="danger"></nb-toggle>
              </div>
            </div>
          </div>
          <div class="col-sm-3" *ngIf="rule.autoKill">
            <div class="form-group mb-0">
              <label class="label">强杀阈值 (分钟)</label>
              <input nbInput fullWidth type="number" [(ngModel)]="rule.autoKillThresholdMinutes" placeholder="默认: 10">
            </div>
          </div>
        </div>

        
        <!-- Channels Configuration -->
        <div class="row mt-3">
            <div class="col-sm-12">
                <div class="form-group">
                    <label class="label d-flex justify-content-between align-items-center">
                        <span>
                            告警媒介配置 (多渠道) 
                            <span class="caption-2 text-hint">默认全天 (00:00 - 24:00)</span>
                        </span>
                        <button nbButton size="tiny" status="success" (click)="addChannel()">
                            <nb-icon icon="plus"></nb-icon> 添加媒介
                        </button>
                    </label>
                    
                    <!-- Channel List -->
                    <div *ngFor="let ch of rule.channels; let i = index" class="channel-card mt-2 p-3 border rounded relative" style="border: 1px solid #edf1f7; background-color: #f7f9fc;">
                         <div class="d-flex justify-content-between mb-2">
                             <div class="d-flex align-items-center gap-2" style="gap: 10px;">
                                 <span class="label">媒介 #{{i+1}}</span>
                                 <nb-select [(ngModel)]="ch.type" (selectedChange)="onChannelTypeChange(ch)" size="small" placeholder="类型" style="width: 100px;">
                                     <nb-option value="tv">TV</nb-option>
                                     <nb-option value="ivr">IVR</nb-option>
                                 </nb-select>
                                 <div class="d-flex align-items-center gap-1">
                                    <input nbInput fieldSize="small" [(ngModel)]="ch.startTime" placeholder="00:00" style="width: 80px;">
                                    <span>-</span>
                                    <input nbInput fieldSize="small" [(ngModel)]="ch.endTime" placeholder="24:00" style="width: 80px;">
                                 </div>
                                 <div class="d-flex align-items-center" style="margin-left: 10px; gap: 8px;">
                                    <span class="caption text-hint" style="white-space: nowrap;">通知间隔:</span>
                                    <input nbInput fieldSize="small" type="number" [(ngModel)]="ch.notifyIntervalMinutes" placeholder="默认: 5" style="width: 100px;" (blur)="validateInterval()">
                                    <span class="caption text-hint">min</span>
                                  </div>
                             </div>
                             <button nbButton ghost size="tiny" status="danger" (click)="removeChannel(i)">
                                 <nb-icon icon="trash-2"></nb-icon>
                             </button>
                         </div>
                         
                         <!-- TV Config -->
                         <div class="row" *ngIf="ch.type === 'tv'">
                             <div class="col-sm-12">
                                 <input nbInput fullWidth fieldSize="small" [(ngModel)]="ch.templateId" placeholder="机器人 Bot ID">
                             </div>
                         </div>
                         
                         <!-- IVR Config -->
                         <div class="row" *ngIf="ch.type === 'ivr'">
                             <div class="col-sm-12">
                                 <div class="form-group mb-2">
                                    <input nbInput fullWidth fieldSize="small" [(ngModel)]="ch.ivrTemplate" [nbAutocomplete]="autoIvr" placeholder="模板 ID (可手动输入)" (ngModelChange)="onChannelTemplateChange(ch, $event)">
                                    <nb-autocomplete #autoIvr>
                                        <nb-option *ngFor="let t of ivrTemplates" [value]="t.id">{{ t.id }}</nb-option>
                                    </nb-autocomplete>
                                    <div class="mt-1 caption-2 text-hint" *ngIf="getChannelTemplateContent(ch)">
                                        {{ getChannelTemplateContent(ch) }}
                                    </div>
                                 </div>
                                 
                                 <label class="label caption-2">模板参数</label>
                                 <div *ngFor="let p of ch.ivrParamRows; let pi = index" class="d-flex mb-1 gap-2" style="gap: 5px;">
                                     <input nbInput fullWidth fieldSize="tiny" [(ngModel)]="p.key" placeholder="Key" style="flex: 1;">
                                     <input nbInput fullWidth fieldSize="tiny" [(ngModel)]="p.value" placeholder="Value" style="flex: 1;">
                                     <button nbButton ghost size="tiny" status="danger" (click)="removeChannelParam(ch, pi)"><nb-icon icon="close"></nb-icon></button>
                                 </div>
                                 <button nbButton ghost size="tiny" status="info" (click)="addChannelParam(ch)">+ 参数</button>
                                 
                                 <div class="form-group mt-2">
                                    <label class="label caption-2">密钥 (Secret)</label>
                                    <input nbInput fullWidth fieldSize="small" [(ngModel)]="ch.ivrSecret" placeholder="默认: 695B539ADBDE6">
                                 </div>
                             </div>
                         </div>
                    </div>
                </div>
            </div>
        </div>

        <div class="row mt-3">
          <div class="col-sm-12">
            <label class="label">接收人 (自动填充邮箱/电话)</label>
            <div *ngFor="let receiver of rule.receivers; let i = index" class="row mb-2">
              <div class="col-sm-2">
                <input nbInput fullWidth [(ngModel)]="receiver.name" placeholder="姓名" (blur)="onNameBlur(receiver)">
              </div>
              <div class="col-sm-4">
                <input nbInput fullWidth [(ngModel)]="receiver.email" placeholder="邮箱" (focus)="onEmailFocus(receiver)">
              </div>
               <div class="col-sm-3">
                <input nbInput fullWidth [(ngModel)]="receiver.phone" placeholder="电话">
              </div>
              <div class="col-sm-2">
                 <nb-select fullWidth [(ngModel)]="receiver.role" placeholder="角色">
                   <nb-option value="duty">值班</nb-option>
                   <nb-option value="manager">主管</nb-option>
                 </nb-select>
              </div>
              <div class="col-sm-1">
                <button nbButton ghost status="danger" (click)="removeReceiver(i)">
                  <nb-icon icon="trash-2"></nb-icon>
                </button>
              </div>
            </div>
            <button nbButton size="tiny" status="primary" (click)="addReceiver()">添加接收人</button>
          </div>
        </div>

        <div class="row mt-4">
          <div class="col-sm-12">
            <div class="form-group d-flex align-items-center">
              <label class="label mb-0 mr-3">启用规则</label>
              <nb-toggle [(ngModel)]="rule.enabled" status="success"></nb-toggle>
            </div>
          </div>
        </div>
      </nb-card-body>
      <nb-card-footer>
        <button nbButton status="primary" (click)="submit()">保存</button>
        <button nbButton ghost (click)="cancel()" class="ml-2">取消</button>
      </nb-card-footer>
    </nb-card>
  `,
  styles: [`
    .rule-detail-card {
        width: 800px;
        max-height: 90vh;
        display: flex;
        flex-direction: column;
        margin: 0;
    }
    nb-card-body {
        flex: 1;
        overflow-y: auto;
    }
    .channel-card {
        position: relative;
    }
  `]
})
export class AlertRuleDetailComponent implements OnInit {
  @Input() rule: AlertRule;

  dataSources: ResourceDataSource[] = [];
  receiverOptions: { [name: string]: Employee[] } = {};

  // 常量定义 IVR 模板
  ivrTemplates = [
    { id: "8BEDD4FD44", content: "你好,系统出现故障,请立即处理." },
    { id: "8C78976734", content: "{$name} 您负责的 {$system} 出现异常,请立即处理!" },
    { id: "7BB0387029", content: "{$name} 您负责的 {$system} 系统 {$function} 功能出现异常,请立即处理!" }
  ];

  constructor(
    protected ref: NbDialogRef<AlertRuleDetailComponent>,
    private clusterService: ClusterService,
    private resourceService: ResourceService,
    private headcountService: HeadcountService,
  ) { }

  ngOnInit() {
    this.resourceService.getDataSources().subscribe(list => {
      this.dataSources = list.filter(ds => ds.type === 'starrocks');
    });

    // Initialize channels if empty
    if (!this.rule.channels || this.rule.channels.length === 0) {
      this.rule.channels = [];
      // Attempt backward compatibility from root fields
      if (this.rule.channel) {
        this.rule.channels.push({
          type: this.rule.channel,
          startTime: '00:00',
          endTime: '24:00',
          templateId: this.rule.templateId,
          ivrTemplate: this.rule.ivrTemplate,
          ivrSecret: this.rule.ivrSecret,
          ivrParams: this.rule.ivrParams,
          notifyIntervalMinutes: this.rule.notifyIntervalMinutes || 5, // Sync global to first channel
          ivrParamRows: [] // will be filled below
        });
      } else {
        // Default new rule: 1 TV channel
        this.rule.channels.push({
          type: 'tv',
          startTime: '00:00',
          endTime: '24:00',
          notifyIntervalMinutes: 5,
          ivrParamRows: []
        });
      }
    }

    // Hydrate params
    this.rule.channels.forEach(ch => {
      if (ch.type === 'ivr') {
        const paramsMap = (ch.ivrParams || {}) as any;
        const rows: { key: string, value: string }[] = [];
        const seenKeys = new Set<string>();

        // 1. If we have a predefined template, lead with its parameters
        const t = this.ivrTemplates.find(x => x.id === ch.ivrTemplate);
        if (t) {
          const matches = t.content.match(/\{\$(\w+)\}/g);
          if (matches) {
            matches.forEach(m => {
              const key = m.replace('{$', '').replace('}', '');
              rows.push({ key: key, value: paramsMap[key] || '' });
              seenKeys.add(key);
            });
          }
        }

        // 2. Add any other existing mapped params that weren't in the template
        Object.keys(paramsMap).forEach(key => {
          if (!seenKeys.has(key)) {
            rows.push({ key: key, value: paramsMap[key] });
            seenKeys.add(key);
          }
        });

        ch.ivrParamRows = rows;
      } else {
        if (!ch.ivrParamRows) ch.ivrParamRows = [];
      }
    });
  }

  getUnit() {
    switch (this.rule.subType) {
      case 'Memory': return 'GB';
      case 'Cpu': return 's';
      case 'ScanRows': return 'Rows';
      case 'ExecutionTime': return 's';
      default: return '';
    }
  }

  onDataSourceChange(id: number) {
    const ds = this.dataSources.find(d => d.id === id);
    if (ds) {
      this.rule.dataSource = ds.name;
    }
  }

  onSubTypeChange() {
  }

  // --- Channel Methods ---

  addChannel() {
    this.rule.channels.push({
      type: 'tv',
      startTime: '00:00',
      endTime: '24:00',
      notifyIntervalMinutes: 5,
      ivrParamRows: []
    });
  }

  removeChannel(index: number) {
    this.rule.channels.splice(index, 1);
  }

  onChannelTypeChange(ch: AlertChannel) {
    if (ch.type === 'ivr' && !ch.ivrTemplate) {
      // Pick first template by default if empty
      this.onChannelTemplateChange(ch, this.ivrTemplates[0].id);
    }
  }

  onChannelTemplateChange(ch: AlertChannel, id: string) {
    ch.ivrTemplate = id; // Update model (input/autocomplete)
    const t = this.ivrTemplates.find(x => x.id === id);
    if (t) {
      // Parse Params from template
      const matches = t.content.match(/\{\$(\w+)\}/g);
      const newRows: { key: string, value: string }[] = [];
      const seenKeys = new Set<string>();

      if (matches) {
        matches.forEach(m => {
          const key = m.replace('{$', '').replace('}', '');
          const exist = ch.ivrParamRows ? ch.ivrParamRows.find(r => r.key === key) : null;
          newRows.push({ key: key, value: exist ? exist.value : '' });
          seenKeys.add(key);
        });
      }

      // Preserve existing manual parameters that don't match the new template
      if (ch.ivrParamRows) {
        ch.ivrParamRows.forEach(row => {
          if (row.key && !seenKeys.has(row.key)) {
            newRows.push(row);
            seenKeys.add(row.key);
          }
        });
      }
      ch.ivrParamRows = newRows;
    } else {
      // Custom template ID: Don't clear parameters immediately as user might be typing a valid ID
      // or using a custom ID with already configured parameters.
    }
  }

  getChannelTemplateContent(ch: AlertChannel) {
    const t = this.ivrTemplates.find(x => x.id === ch.ivrTemplate);
    return t ? t.content : '';
  }

  addChannelParam(ch: AlertChannel) {
    if (!ch.ivrParamRows) ch.ivrParamRows = [];
    ch.ivrParamRows.push({ key: '', value: '' });
  }

  removeChannelParam(ch: AlertChannel, index: number) {
    ch.ivrParamRows.splice(index, 1);
  }

  // --- Receiver Methods ---

  onNameBlur(receiver: AlertReceiver) {
    if (receiver.name) {
      this.lookupName(receiver.name, (employees) => {
        if (employees.length === 1) {
          if (!receiver.email) receiver.email = employees[0].email; // Only set if empty? User might prefer overwriting. Logic in prev file overwritten.
          receiver.email = employees[0].email;
          receiver.phone = employees[0].phone;
        }
      });
    }
  }

  onEmailFocus(receiver: AlertReceiver) {
    if (receiver.name) {
      this.lookupName(receiver.name);
    }
  }

  lookupName(name: string, callback?: (emps: Employee[]) => void) {
    if (this.receiverOptions[name]) {
      if (callback) callback(this.receiverOptions[name]);
      return;
    }

    this.headcountService.listEmployees(1, 50, name).pipe(
      map(res => res.data.list.filter(e => e.name === name || e.userId === name))
    ).subscribe(employees => {
      this.receiverOptions[name] = employees;
      if (callback) callback(employees);
    });
  }

  addReceiver() {
    this.rule.receivers.push({ name: '', email: '', role: 'duty', phone: '' });
  }

  removeReceiver(index: number) {
    this.rule.receivers.splice(index, 1);
  }

  cancel() {
    this.ref.close();
  }

  validateInterval() {
    if (this.rule.channels) {
      this.rule.channels.forEach(ch => {
        // Enforce default 5
        if (!ch.notifyIntervalMinutes || ch.notifyIntervalMinutes <= 0) {
          ch.notifyIntervalMinutes = 5;
        }
        // Enforce IVR 3min min
        if (ch.type === 'ivr' && ch.notifyIntervalMinutes < 3) {
          ch.notifyIntervalMinutes = 3;
        }
      });
    }
  }

  submit() {
    this.validateInterval();
    // Process channels to save params
    if (this.rule.channels) {
      this.rule.channels.forEach(ch => {
        if (ch.type === 'ivr' && ch.ivrParamRows) {
          const params: any = {};
          ch.ivrParamRows.forEach(row => {
            if (row.key) params[row.key] = row.value;
          });
          ch.ivrParams = params;
        }
      });

      // Sync first channel to root props for legacy compatibility
      if (this.rule.channels.length > 0) {
        const first = this.rule.channels[0];
        this.rule.channel = first.type;
        this.rule.templateId = first.templateId;
        this.rule.ivrTemplate = first.ivrTemplate;
        this.rule.ivrSecret = first.ivrSecret;
        this.rule.ivrParams = first.ivrParams;
        this.rule.notifyIntervalMinutes = first.notifyIntervalMinutes || 5;
      }
    }

    this.ref.close(this.rule);
  }
}

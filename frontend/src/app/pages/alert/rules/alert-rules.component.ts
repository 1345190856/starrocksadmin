import { Component, OnInit } from '@angular/core';
import { NbDialogService, NbToastrService } from '@nebular/theme';
import { AlertService, AlertRule } from '../../../@core/services/alert.service';
import { AlertRuleDetailComponent } from './alert-rule-detail.component';

@Component({
  selector: 'ngx-alert-rules',
  template: `
    <nb-card>
      <nb-card-header>
        告警规则管理
        <button nbButton size="small" status="primary" style="float: right;" (click)="createRule()">新增规则</button>
      </nb-card-header>
      <nb-card-body>
        <table class="table table-striped">
          <thead>
            <tr>
              <th>规则名称</th>
              <th>地区</th>
              <th>数据源</th>
              <th>类型</th>
              <th>阈值</th>
              <th>状态</th>
              <th>操作</th>
            </tr>
          </thead>
          <tbody>
            <tr *ngFor="let rule of rules">
              <td>{{ rule.name }}</td>
              <td>{{ rule.region }}</td>
              <td>{{ rule.dataSource }}</td>
              <td>{{ rule.subType }}</td>
              <td>{{ formatThreshold(rule) }}</td>
              <td>
                <nb-tag *ngIf="rule.enabled" status="success" text="已启用" size="tiny" (click)="toggleRuleStatus(rule)" style="cursor: pointer;"></nb-tag>
                <nb-tag *ngIf="!rule.enabled" status="danger" text="已禁用" size="tiny" (click)="toggleRuleStatus(rule)" style="cursor: pointer;"></nb-tag>
              </td>
              <td>
                <button nbButton ghost size="tiny" status="info" (click)="editRule(rule)" title="编辑">
                  <nb-icon icon="edit"></nb-icon>
                </button>
                <button nbButton ghost size="tiny" status="primary" (click)="cloneRule(rule)" title="克隆">
                  <nb-icon icon="copy"></nb-icon>
                </button>
                <button nbButton ghost size="tiny" status="warning" (click)="testRule(rule)" title="点击测试告警发送">
                  <nb-icon icon="bell-outline"></nb-icon>
                </button>
                <button nbButton ghost size="tiny" status="danger" (click)="deleteRule(rule)" title="删除">
                  <nb-icon icon="trash-2"></nb-icon>
                </button>
              </td>
            </tr>
            <tr *ngIf="rules.length === 0">
              <td colspan="7" class="text-center">暂无规则</td>
            </tr>
          </tbody>
        </table>
      </nb-card-body>
    </nb-card>
  `,
  styles: [`
    .table td {
      vertical-align: middle;
    }
    /* Remove sticky focus box-shadow/outline on action buttons */
    ::ng-deep .table td button.nb-button:focus,
    ::ng-deep .table td button.nb-button:active,
    ::ng-deep .table td button.nb-button:focus-visible {
      box-shadow: none !important;
      outline: none !important;
      -webkit-box-shadow: none !important;
    }
    /* Ensure hover still works if desired, but remove focus stickiness */
  `]
})
export class AlertRulesComponent implements OnInit {
  rules: AlertRule[] = [];

  constructor(
    private alertService: AlertService,
    private dialogService: NbDialogService,
    private toastr: NbToastrService
  ) { }

  ngOnInit() {
    this.loadRules();
  }

  loadRules() {
    this.alertService.getRules().subscribe(rules => {
      // Handle camelCase mapping if needed, but assuming API handles it
      this.rules = rules;
    });
  }

  createRule() {
    this.dialogService.open(AlertRuleDetailComponent, {
      context: {
        rule: {
          receivers: [
            { name: '', email: '', role: 'duty' }
          ],
          enabled: true,
          autoKill: false,
          autoKillThresholdMinutes: 10,
          notifyIntervalMinutes: 5,
          alertType: 'Abnormal SQL',
          starrocksVersion: '3.3'
        } as any
      }
    }).onClose.subscribe(newRule => {
      if (newRule) {
        this.alertService.createRule(newRule).subscribe(() => {
          this.toastr.success('创建成功', '成功');
          this.loadRules();
        });
      }
    });
  }

  editRule(rule: AlertRule) {
    this.dialogService.open(AlertRuleDetailComponent, {
      context: {
        rule: JSON.parse(JSON.stringify(rule))
      }
    }).onClose.subscribe(updatedRule => {
      if (updatedRule) {
        this.alertService.updateRule(rule.id, updatedRule).subscribe(() => {
          this.toastr.success('更新成功', '成功');
          this.loadRules();
        });
      }
    });
  }

  cloneRule(rule: AlertRule) {
    const cloned = JSON.parse(JSON.stringify(rule));
    cloned.id = undefined;
    cloned.name = `${rule.name} (Copy)`;
    this.dialogService.open(AlertRuleDetailComponent, {
      context: {
        rule: cloned
      }
    }).onClose.subscribe(newRule => {
      if (newRule) {
        this.alertService.createRule(newRule).subscribe(() => {
          this.toastr.success('克隆成功', '成功');
          this.loadRules();
        });
      }
    });
  }

  testRule(rule: AlertRule) {
    this.alertService.testRule(rule.id).subscribe(() => {
      this.toastr.success('测试告警指令已发出，请检查接收端 (botId: ' + (rule.templateId || 'N/A') + ')', '成功');
    }, err => {
      this.toastr.danger('发送失败: ' + (err.error?.message || '未知错误'), '错误');
    });
  }

  deleteRule(rule: AlertRule) {
    if (confirm(`确定删除规则 "${rule.name}" 吗?`)) {
      this.alertService.deleteRule(rule.id).subscribe(() => {
        this.toastr.success('删除成功', '成功');
        this.loadRules();
      });
    }
  }

  toggleRuleStatus(rule: AlertRule) {
    const updatedStatus = !rule.enabled;
    this.alertService.updateRule(rule.id, { enabled: updatedStatus } as any).subscribe(() => {
      rule.enabled = updatedStatus;
      this.toastr.success(updatedStatus ? '规则已启用' : '规则已禁用', '成功');
    }, err => {
      this.toastr.danger('操作失败: ' + (err.error?.message || '未知错误'), '错误');
    });
  }

  formatThreshold(rule: AlertRule): string {
    switch (rule.subType) {
      case 'Memory': return rule.threshold + ' GB';
      case 'Cpu': return rule.threshold + ' s';
      case 'ScanRows': return rule.threshold + ' Rows';
      case 'ExecutionTime': return rule.threshold + ' s';
      default: return rule.threshold + '';
    }
  }
}

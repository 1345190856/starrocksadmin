import { Component, Input, OnInit } from '@angular/core';
import { NbDialogRef } from '@nebular/theme';
import { DutyService } from '../../../@core/services/duty.service';

@Component({
  selector: 'ngx-notify-dialog',
  template: `
    <nb-card style="min-width: 450px;">
      <nb-card-header>推送配置 ({{ platformName }})</nb-card-header>
      <nb-card-body>
        <div class="form-group">
          <label class="label">Webhook Bot IDs (支持多个，逗号或换行分隔)</label>
          <textarea nbInput fullWidth [(ngModel)]="botIds" placeholder="请输入 Bot ID" rows="3"></textarea>
        </div>
        
        <div class="form-group" style="margin-top: 1rem;">
          <div style="display: flex; align-items: center; justify-content: space-between;">
            <div style="display: flex; align-items: center; gap: 10px;">
              <label class="label" style="margin-bottom: 0;">自动通知</label>
              <input nbInput type="number" [(ngModel)]="notifyAdvanceHours" style="width: 60px; height: 28px;" placeholder="小时" size="small">
            </div>
            <nb-toggle [(ngModel)]="autoNotify" status="success"></nb-toggle>
          </div>
          <p style="font-size: 11px; color: #8f9bb3; margin: 5px 0 0 0;">截止日期前{{ notifyAdvanceHours }}小时自动发送下一周期预告</p>
        </div>
      </nb-card-body>
      <nb-card-footer style="display: flex; justify-content: space-between;">
        <button nbButton status="danger" ghost (click)="cancel()">取消</button>
        <div>
          <button nbButton status="info" (click)="save()" style="margin-right: 0.5rem;">保存</button>
          <button nbButton status="success" (click)="send()">发送通知</button>
        </div>
      </nb-card-footer>
    </nb-card>
  `,
})
export class NotifyDialogComponent implements OnInit {
  @Input() platformName: string = '数据平台';
  botIds: string = '';
  autoNotify: boolean = false;
  notifyAdvanceHours: number = 7;

  constructor(
    protected ref: NbDialogRef<NotifyDialogComponent>,
    private dutyService: DutyService
  ) { }

  ngOnInit() {
    // Load existing config from DB
    this.dutyService.getRotations().subscribe(rotations => {
      let rot = rotations.find(r => r.name === this.platformName);

      // 如果是“所有平台”且没找到配置，尝试找一个已有的配置（因为它们通常是共用的）
      if (!rot && this.platformName === '所有平台') {
        rot = rotations.find(r => r.name === '数据平台') || rotations.find(r => r.name === '数仓');
      }

      if (rot) {
        this.botIds = rot.bot_ids || '';
        this.autoNotify = rot.auto_notify || false;
        this.notifyAdvanceHours = rot.notify_advance_hours || 7;
      } else {
        this.botIds = localStorage.getItem('last_duty_bot_id') || '';
      }
    });
  }

  cancel() {
    this.ref.close();
  }

  save(shouldClose: boolean = true) {
    const names = this.platformName === '所有平台' ? ['数据平台', '数仓'] : [this.platformName];

    names.forEach(name => {
      this.dutyService.updateRotationConfig({
        name: name,
        bot_ids: this.botIds,
        auto_notify: this.autoNotify,
        notify_advance_hours: this.notifyAdvanceHours
      }).subscribe();
    });

    localStorage.setItem('last_duty_bot_id', this.botIds);
    if (shouldClose) {
      this.ref.close();
    }
  }

  send() {
    if (!this.botIds) return;

    // 先保存配置（不关闭弹窗）
    this.save(false);

    // 返回 IDs 给父组件以触发通知发送，并关闭弹窗
    const ids = this.botIds.split(/[\s,，]+/).filter(id => !!id.trim());
    this.ref.close(ids);
  }
}

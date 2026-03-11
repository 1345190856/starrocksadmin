import { Component, OnInit, OnDestroy } from '@angular/core';
import { DutyService, DutySchedule } from '../../../../@core/services/duty.service';
import { interval, Subscription } from 'rxjs';

@Component({
    selector: 'ngx-duty-barrage',
    templateUrl: './duty-barrage.component.html',
    styleUrls: ['./duty-barrage.component.scss']
})
export class DutyBarrageComponent implements OnInit, OnDestroy {
    dutyItems: string[] = [];
    title: string = '本周值班';
    private refreshSub: Subscription;
    private timerSub: Subscription;

    constructor(private dutyService: DutyService) { }

    ngOnInit() {
        this.loadDutyInfo();
        // 每10分钟刷新一次
        this.timerSub = interval(600000).subscribe(() => this.loadDutyInfo());

        // 监听手动刷新信号
        this.refreshSub = this.dutyService.refreshSchedule$.subscribe(() => {
            this.loadDutyInfo();
        });
    }

    ngOnDestroy() {
        if (this.timerSub) this.timerSub.unsubscribe();
        if (this.refreshSub) this.refreshSub.unsubscribe();
    }

    loadDutyInfo() {
        const today = new Date();
        today.setHours(0, 0, 0, 0);

        this.dutyService.getRotations().subscribe(rotations => {
            // 1. 寻找当前生效的排班（按创建时间倒序，找第一个包含今天的）
            const relevantRotations = (rotations || []).filter(r => r.name === '数据平台' || r.name === '数仓');
            let activeRotation = null;

            for (const r of relevantRotations) {
                const start = new Date(r.start_date);
                const end = new Date(r.end_date);
                if (today >= start && today <= end) {
                    activeRotation = r;
                    break;
                }
            }

            let startStr: string, endStr: string;
            if (activeRotation) {
                this.title = `${activeRotation.name}排班`;

                // 裁切逻辑：如果配置范围过长（如一年），仅展示合理窗口
                let start = new Date(activeRotation.start_date);
                let end = new Date(activeRotation.end_date);

                const diffDays = (end.getTime() - start.getTime()) / (1000 * 60 * 60 * 24);
                if (diffDays > 31) { // 超过一个月就裁切
                    const limitStart = new Date(today);
                    const limitEnd = new Date(today);
                    limitEnd.setDate(today.getDate() + 7); // 展示接下来一周

                    if (start < limitStart) start = limitStart;
                    if (end > limitEnd) end = limitEnd;
                }

                startStr = this.formatDate(start);
                endStr = this.formatDate(end);
            } else {
                // 如果没有包含今天的Rotation，显示本周
                this.title = '本周值班';
                const dayOfWeek = today.getDay();
                const monday = new Date(today);
                monday.setDate(today.getDate() - (dayOfWeek === 0 ? 6 : dayOfWeek - 1));
                const sunday = new Date(monday);
                sunday.setDate(monday.getDate() + 6);

                startStr = this.formatDate(monday);
                endStr = this.formatDate(sunday);
            }

            this.dutyService.getSchedule({ start_date: startStr, end_date: endStr }).subscribe(data => {
                this.processDutyData(data);
            });
        });
    }

    processDutyData(data: DutySchedule[]) {
        if (!data || data.length === 0) {
            this.dutyItems = ['暂无近期排班信息'];
            return;
        }

        const grouped: { [date: string]: string } = {};
        data.forEach(item => {
            if (item.personnel_name) {
                const platform = item.duty_platform || '通用';
                const person = `${item.personnel_name}(${platform})`;
                if (!grouped[item.duty_date]) {
                    grouped[item.duty_date] = person;
                } else if (!grouped[item.duty_date].split(', ').includes(person)) {
                    grouped[item.duty_date] += `, ${person}`;
                }
            }
        });

        const dates = Object.keys(grouped).sort();
        const merged: string[] = [];

        if (dates.length > 0) {
            let start = dates[0];
            let content = grouped[start];

            for (let i = 1; i <= dates.length; i++) {
                const current = dates[i];
                const prev = dates[i - 1];

                const isConsecutive = current && (new Date(current).getTime() - new Date(prev).getTime() === 86400000);
                const isSameContent = current && grouped[current] === content;

                if (isConsecutive && isSameContent) {
                    continue;
                } else {
                    const rangeEnd = dates[i - 1];
                    const sDisp = start.substring(5); // MM-DD
                    const eDisp = rangeEnd.substring(5);

                    if (start === rangeEnd) {
                        merged.push(`${sDisp}: ${content}`);
                    } else {
                        merged.push(`${sDisp} ~ ${eDisp}: ${content}`);
                    }

                    if (current) {
                        start = current;
                        content = grouped[current];
                    }
                }
            }
        }

        this.dutyItems = merged;
    }

    formatDate(d: Date): string {
        const year = d.getFullYear();
        const month = ('0' + (d.getMonth() + 1)).slice(-2);
        const day = ('0' + d.getDate()).slice(-2);
        return `${year}-${month}-${day}`;
    }
}

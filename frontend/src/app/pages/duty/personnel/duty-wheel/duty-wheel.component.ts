import { Component, OnInit, Input, ChangeDetectorRef } from '@angular/core';
import { NbDialogRef, NbToastrService } from '@nebular/theme';
import { DutyService, DutyPersonnel, DutyRotation, DutySchedule } from '../../../../@core/services/duty.service';
import { CdkDragDrop, moveItemInArray } from '@angular/cdk/drag-drop';
import { forkJoin, of } from 'rxjs';
import { catchError } from 'rxjs/operators';

@Component({
    selector: 'ngx-duty-wheel',
    templateUrl: './duty-wheel.component.html',
    styleUrls: ['./duty-wheel.component.scss']
})
export class DutyWheelComponent implements OnInit {
    @Input() personnel: DutyPersonnel[] = [];

    activePlatform: string = '数据平台';
    platforms = ['数据平台', '数仓'];
    availablePersonnel: DutyPersonnel[] = [];

    startDate: Date = new Date();
    endDate: Date = new Date();
    periodDays: number = 7;

    allRotations: DutyRotation[] = [];
    colors = ['#3366ff', '#00d68f', '#ffaa00', '#ff3d71', '#a16eff', '#0095ff', '#2ce6f1', '#6ded2d'];

    isDragging = false;
    isSpinning = false;
    currentAngle = 0;
    animationAngle = 0;
    manualSectorIndex: number | null = null;

    platformStates: {
        [key: string]: {
            startDate: Date,
            endDate: Date,
            periodDays: number,
            availablePersonnel: DutyPersonnel[],
            manualSectorIndex: number | null,
            currentAngle: number
        }
    } = {};

    constructor(
        protected ref: NbDialogRef<DutyWheelComponent>,
        private dutyService: DutyService,
        private cdr: ChangeDetectorRef,
        private toastrService: NbToastrService
    ) {
        this.endDate.setDate(this.startDate.getDate() + 6);
    }

    ngOnInit() {
        if (!this.personnel) this.personnel = [];
        this.loadData();
    }

    /**
     * 同时加载 duty_rotation 和 duty_schedule 数据
     * 如果 rotation 失败，降级到 schedule；如果都失败，给出具体原因
     */
    loadData() {
        // 用一个足够大的日期范围来获取所有当前的 schedule
        const wideStart = '2026-01-01';
        const wideEnd = '2027-01-01';

        forkJoin({
            rotations: this.dutyService.getRotations().pipe(
                catchError(err => {
                    console.error('加载值班轮换配置失败:', err);
                    this.toastrService.warning(
                        `轮换配置加载失败: ${err?.error?.message || err?.message || '网络错误'}，尝试从排班表获取数据`,
                        '警告'
                    );
                    return of([] as DutyRotation[]);
                })
            ),
            schedules: this.dutyService.getSchedule({ start_date: wideStart, end_date: wideEnd }).pipe(
                catchError(err => {
                    console.error('加载值班排班数据失败:', err);
                    return of([] as DutySchedule[]);
                })
            )
        }).subscribe({
            next: ({ rotations, schedules }) => {
                this.allRotations = rotations || [];

                // 对每个平台初始化状态
                this.platforms.forEach(p => this.initPlatformState(p, schedules || []));
                this.loadFromState(this.activePlatform);

                if (this.availablePersonnel.length > 0) {
                    this.startLuckyDraw();
                }

                // 如果两个数据源都为空，给出详细错误
                if ((rotations || []).length === 0 && (schedules || []).length === 0) {
                    this.toastrService.danger(
                        '值班轮换配置和排班表均无数据，请检查后端数据库连接及 duty_rotation / duty_schedule 表',
                        '数据加载失败'
                    );
                }

                this.cdr.detectChanges();
            },
            error: (err) => {
                console.error('值班数据加载异常:', err);
                this.toastrService.danger(
                    `无法连接后端服务: ${err?.message || '未知错误'}`,
                    '严重错误'
                );
            }
        });
    }

    /**
     * 初始化平台状态：优先用 rotation + schedule 数据，保证时间跨度和 duty_schedule.duty_date 一致
     */
    private initPlatformState(platform: string, allSchedules: DutySchedule[]) {
        // 1. 过滤出该平台的人员
        const platformPersonnel = this.personnel.filter(p => {
            if (!p.duty_platform) return false;
            const pPlatform = p.duty_platform.trim();
            const targetPlatform = platform.trim();
            return (pPlatform === targetPlatform || (targetPlatform === '数据平台' && pPlatform === 'all'))
                && pPlatform !== '无';
        });

        // 2. 过滤出该平台的排班记录
        const platformSchedules = allSchedules.filter(s =>
            s.duty_platform && s.duty_platform.trim() === platform.trim()
        );

        // 3. 查找该平台的轮换配置
        const rotation = (this.allRotations || []).find(r => r.name && r.name.trim() === platform.trim());

        let periodDays = 7;
        let startDate = new Date();
        let endDate = new Date();
        let availablePersonnel: DutyPersonnel[] = [];

        // 4. 确定日期范围：优先从 duty_schedule 获取实际日期，其次用 rotation 配置
        if (platformSchedules.length > 0) {
            // 从 schedule 的实际 duty_date 获取时间跨度
            const dates = platformSchedules.map(s => this.parseDate(s.duty_date)).filter(d => d !== null) as Date[];
            dates.sort((a, b) => a.getTime() - b.getTime());
            startDate = new Date(dates[0]);
            endDate = new Date(dates[dates.length - 1]);
            periodDays = Math.round((endDate.getTime() - startDate.getTime()) / (1000 * 60 * 60 * 24)) + 1;
        } else if (rotation) {
            // schedule 为空但 rotation 配置存在，直接使用 rotation 的 start_date 和 end_date
            startDate = this.parseDate(rotation.start_date) || new Date();
            endDate = this.parseDate(rotation.end_date) || new Date();
            periodDays = (rotation.period_days != null) ? rotation.period_days : 7;
        } else {
            // 两个都没有，使用默认值
            startDate.setHours(0, 0, 0, 0);
            endDate = new Date(startDate.getTime());
            endDate.setDate(startDate.getDate() + periodDays - 1);
        }

        // 5. 确定人员列表及顺序
        if (rotation) {
            try {
                const ids: number[] = rotation.personnel_ids ? JSON.parse(rotation.personnel_ids) : [];
                const savedList = ids
                    .map(id => platformPersonnel.find(p => p.id === id))
                    .filter(p => !!p) as DutyPersonnel[];
                const remaining = platformPersonnel.filter(p => !ids.includes(p.id));
                availablePersonnel = [...savedList, ...remaining];
            } catch (e) {
                availablePersonnel = [...platformPersonnel];
            }
        } else {
            availablePersonnel = [...platformPersonnel];
        }

        this.platformStates[platform] = {
            startDate, endDate, periodDays, availablePersonnel,
            manualSectorIndex: null,
            currentAngle: 0
        };
    }

    onPlatformChange(platform: string) {
        this.saveToState(this.activePlatform);
        this.activePlatform = platform;
        this.loadFromState(platform);

        if (this.availablePersonnel.length > 0) {
            this.startLuckyDraw();
        }
    }

    private saveToState(platform: string) {
        this.platformStates[platform] = {
            startDate: new Date(this.startDate),
            endDate: new Date(this.endDate),
            periodDays: this.periodDays,
            availablePersonnel: [...this.availablePersonnel],
            manualSectorIndex: this.manualSectorIndex,
            currentAngle: this.currentAngle
        };
    }

    private loadFromState(platform: string) {
        const state = this.platformStates[platform];
        if (state) {
            this.startDate = state.startDate;
            this.endDate = state.endDate;
            this.periodDays = state.periodDays;
            this.availablePersonnel = state.availablePersonnel;
            this.manualSectorIndex = state.manualSectorIndex;
            this.currentAngle = state.currentAngle;
            this.cdr.detectChanges();
        }
    }

    getLogicalCurrentIndex(): number {
        const n = this.availablePersonnel.length;
        if (n === 0) return 0;

        const today = new Date();
        today.setHours(0, 0, 0, 0);
        const start = this.parseDate(this.startDate);
        if (start) start.setHours(0, 0, 0, 0);

        if (!start || today < start) return 0;

        const diffTime = today.getTime() - start.getTime();
        const diffDays = Math.floor(diffTime / (1000 * 60 * 60 * 24));
        const period = this.periodDays || 1;
        return Math.floor(diffDays / period) % n;
    }

    private parseDate(d: any): Date | null {
        if (!d) return null;
        if (d instanceof Date) return isNaN(d.getTime()) ? null : d;
        if (typeof d === 'string') {
            const parts = d.split('T')[0].split('-');
            if (parts.length === 3) {
                const y = parseInt(parts[0], 10);
                const m = parseInt(parts[1], 10) - 1;
                const date = parseInt(parts[2], 10);
                const dt = new Date(y, m, date);
                return isNaN(dt.getTime()) ? null : dt;
            }
        }
        const dt = new Date(d);
        return isNaN(dt.getTime()) ? null : dt;
    }

    startLuckyDraw() {
        if (this.availablePersonnel.length === 0) return;

        this.isSpinning = true;
        const duration = 1200;
        const startTime = performance.now();
        const startAngle = this.currentAngle;

        const n = this.availablePersonnel.length;
        const currentIdx = this.getLogicalCurrentIndex();
        const anglePerSector = 360 / n;
        const targetAngle = currentIdx * anglePerSector + anglePerSector / 2;

        const totalSpins = 1;
        const endAngle = (totalSpins * 360) + targetAngle;

        const animate = (time: number) => {
            const elapsed = time - startTime;
            const progress = Math.min(elapsed / duration, 1);
            const easeOutProgress = 1 - Math.pow(1 - progress, 3);

            this.animationAngle = startAngle + (endAngle - startAngle) * easeOutProgress;
            this.cdr.detectChanges();

            if (progress < 1) {
                requestAnimationFrame(animate);
            } else {
                this.isSpinning = false;
                this.currentAngle = endAngle % 360;
                this.manualSectorIndex = null;
                this.cdr.detectChanges();
            }
        };
        requestAnimationFrame(animate);
    }

    onDateChange() {
        this.onPeriodChange();
    }

    onPeriodChange() {
        const start = this.parseDate(this.startDate);
        const period = Number(this.periodDays);
        if (start && period > 0) {
            const end = new Date(start.getTime());
            end.setDate(start.getDate() + period - 1);
            this.endDate = end;
            this.cdr.detectChanges();
        }
    }

    drop(event: CdkDragDrop<DutyPersonnel[]>) {
        moveItemInArray(this.availablePersonnel, event.previousIndex, event.currentIndex);
        this.manualSectorIndex = null;
        this.cdr.detectChanges();
    }

    onMouseDown(event: MouseEvent) {
        if (this.isSpinning) return;
        event.stopPropagation();
        event.preventDefault();
        this.isDragging = true;
        this.updateAngleFromEvent(event);
        window.addEventListener('mousemove', this.onMouseMove, { capture: true });
        window.addEventListener('mouseup', this.onMouseUp, { capture: true });
    }

    onMouseMove = (event: MouseEvent) => {
        if (!this.isDragging) return;
        this.updateAngleFromEvent(event);
    }

    private updateAngleFromEvent(event: MouseEvent) {
        const wheel = document.querySelector('.wheel-clock-container');
        if (!wheel) return;

        const rect = wheel.getBoundingClientRect();
        const centerX = rect.left + rect.width / 2;
        const centerY = rect.top + rect.height / 2;

        const dx = event.clientX - centerX;
        const dy = event.clientY - centerY;

        let angle = Math.atan2(dy, dx) * (180 / Math.PI);
        angle = (angle + 90 + 360) % 360;

        const n = this.availablePersonnel.length;
        if (n > 0) {
            const anglePerSector = 360 / n;
            this.manualSectorIndex = Math.floor(angle / anglePerSector) % n;
            this.currentAngle = angle;
        }
        this.cdr.detectChanges();
    }

    onMouseUp = () => {
        if (this.isDragging) {
            this.isDragging = false;
            if (this.manualSectorIndex !== null) {
                const n = this.availablePersonnel.length;
                const anglePerSector = 360 / n;
                this.currentAngle = this.manualSectorIndex * anglePerSector + anglePerSector / 2;
            }
        }
        window.removeEventListener('mousemove', this.onMouseMove, { capture: true });
        window.removeEventListener('mouseup', this.onMouseUp, { capture: true });
        this.cdr.detectChanges();
    }

    cancel() {
        this.ref.close();
    }

    save() {
        this.saveToState(this.activePlatform);

        const savePayloads = this.platforms.map(platform => {
            const state = this.platformStates[platform];
            if (!state || state.availablePersonnel.length === 0) return null;

            let finalPersonnel = [...state.availablePersonnel];
            if (state.manualSectorIndex !== null) {
                const n = finalPersonnel.length;
                const today = new Date();
                today.setHours(0, 0, 0, 0);
                const start = new Date(state.startDate);
                start.setHours(0, 0, 0, 0);

                let intendedIdx = 0;
                if (start && today >= start) {
                    const diffTime = today.getTime() - start.getTime();
                    const diffDays = Math.floor(diffTime / (1000 * 60 * 60 * 24));
                    const period = state.periodDays || 1;
                    intendedIdx = Math.floor(diffDays / period) % n;
                }

                if (state.manualSectorIndex !== intendedIdx) {
                    const shiftAmt = (state.manualSectorIndex - intendedIdx + n) % n;
                    for (let i = 0; i < shiftAmt; i++) {
                        const p = finalPersonnel.shift();
                        if (p) finalPersonnel.push(p);
                    }
                }
            }

            return {
                name: platform,
                personnel_ids: finalPersonnel.map(p => p.id),
                start_date: this.formatDate(state.startDate),
                end_date: this.formatDate(state.endDate),
                period_days: state.periodDays,
                country: 'all'
            };
        }).filter(p => !!p);

        if (savePayloads.length === 0) return;

        import('rxjs').then(({ forkJoin }) => {
            const requests = savePayloads.map(payload => this.dutyService.saveRotation(payload as any));
            forkJoin(requests).subscribe({
                next: () => {
                    this.toastrService.success('所有平台排班配置保存成功', '成功');
                    this.dutyService.triggerRefreshSchedule();
                    this.ref.close(true);
                },
                error: (err) => {
                    console.error('Failed to save rotations:', err);
                    this.toastrService.danger('保存失败: ' + (err.error?.message || '未知错误'), '错误');
                }
            });
        });
    }

    formatDate(d: Date): string {
        const year = d.getFullYear();
        const month = ('0' + (d.getMonth() + 1)).slice(-2);
        const day = ('0' + d.getDate()).slice(-2);
        return `${year}-${month}-${day}`;
    }

    getSectors() {
        const n = this.availablePersonnel.length;
        if (n === 0) return [];
        const sectors = [];
        const angle = 360 / n;
        for (let i = 0; i < n; i++) {
            const startAngle = i * angle;
            const endAngle = (i + 1) * angle;
            sectors.push({
                ...this.availablePersonnel[i],
                color: this.colors[i % this.colors.length],
                d: this.describeArc(190, 190, 180, startAngle, endAngle)
            });
        }
        return sectors;
    }

    getLabelStyle(i: number) {
        const n = this.availablePersonnel.length;
        if (n === 0) return {};
        const anglePerSector = 360 / n;
        const sectorAngle = i * anglePerSector + anglePerSector / 2;
        const radius = 90;
        const rad = (sectorAngle - 90) * Math.PI / 180;
        const x = 190 + radius * Math.cos(rad);
        const y = 190 + radius * Math.sin(rad);
        return {
            left: `${x}px`,
            top: `${y}px`
        };
    }

    polarToCartesian(centerX: number, centerY: number, radius: number, angleInDegrees: number) {
        const angleInRadians = (angleInDegrees - 90) * Math.PI / 180.0;
        return {
            x: centerX + (radius * Math.cos(angleInRadians)),
            y: centerY + (radius * Math.sin(angleInRadians))
        };
    }

    describeArc(x: number, y: number, radius: number, startAngle: number, endAngle: number) {
        const start = this.polarToCartesian(x, y, radius, endAngle);
        const end = this.polarToCartesian(x, y, radius, startAngle);
        const largeArcFlag = endAngle - startAngle <= 180 ? "0" : "1";
        return [
            "M", x, y,
            "L", start.x, start.y,
            "A", radius, radius, 0, largeArcFlag, 0, end.x, end.y,
            "L", x, y,
            "Z"
        ].join(" ");
    }

    getNeedleRotation(): string {
        if (this.isSpinning) return `rotate(${this.animationAngle}deg)`;
        if (this.isDragging || this.manualSectorIndex !== null) {
            return `rotate(${this.currentAngle}deg)`;
        }
        const n = this.availablePersonnel.length;
        if (n === 0) return 'rotate(0deg)';
        const currentIdx = this.getLogicalCurrentIndex();
        const anglePerSector = 360 / n;
        return `rotate(${currentIdx * anglePerSector + anglePerSector / 2}deg)`;
    }
}

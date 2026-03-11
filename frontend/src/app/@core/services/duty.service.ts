import { Injectable } from '@angular/core';
import { ApiService } from '../data/api.service';
import { Observable, Subject } from 'rxjs';

export interface DutyPersonnel {
    id: number;
    name: string;
    org_lvl1?: string;
    org_lvl2?: string;
    email: string;
    phone: string;
    duty_platform?: string;
    responsible_domain?: string;
    countries?: string;
}

export interface DutySchedule {
    id: number;
    duty_date: string;
    country: string;
    shift_type: string;
    personnel_id: number;
    personnel_name?: string;
    personnel_email?: string;
    duty_platform?: string;
}

export interface DutyRotation {
    id: number;
    name: string;
    personnel_ids: string; // JSON string in backend, but we'll parse it
    start_date: string;
    end_date: string;
    period_days: number;
    country: string;
    bot_ids?: string;
    auto_notify?: boolean;
    notify_advance_hours?: number;
}

@Injectable({
    providedIn: 'root'
})
export class DutyService {
    private refreshScheduleSubject = new Subject<void>();
    refreshSchedule$ = this.refreshScheduleSubject.asObservable();

    triggerRefreshSchedule() {
        this.refreshScheduleSubject.next();
    }

    constructor(private api: ApiService) { }

    getPersonnel(): Observable<DutyPersonnel[]> {
        return this.api.get('/duty/personnel');
    }

    createPersonnel(data: Partial<DutyPersonnel>): Observable<DutyPersonnel> {
        return this.api.post('/duty/personnel', data);
    }

    updatePersonnel(id: number, data: Partial<DutyPersonnel>): Observable<DutyPersonnel> {
        return this.api.put(`/duty/personnel/${id}`, data);
    }

    deletePersonnel(id: number): Observable<any> {
        return this.api.delete(`/duty/personnel/${id}`);
    }

    getSchedule(params: { start_date: string; end_date: string; country?: string; duty_platform?: string }): Observable<DutySchedule[]> {
        return this.api.get('/duty/schedule', { params });
    }

    batchAssignSchedule(schedules: { duty_date: string; country: string; shift_type: string; personnel_ids: number[] }[]): Observable<any> {
        return this.api.post('/duty/schedule/batch', { schedules });
    }

    getRotations(): Observable<DutyRotation[]> {
        return this.api.get('/duty/rotation');
    }

    saveRotation(data: Partial<DutyRotation>): Observable<DutyRotation> {
        return this.api.post('/duty/rotation', data);
    }

    updateRotationConfig(data: { name: string; bot_ids?: string; auto_notify: boolean; notify_advance_hours: number }): Observable<any> {
        return this.api.put('/duty/rotation/config', data);
    }

    notifyManual(bot_ids: string[]): Observable<any> {
        return this.api.post('/duty/notify-manual', { bot_ids });
    }
}

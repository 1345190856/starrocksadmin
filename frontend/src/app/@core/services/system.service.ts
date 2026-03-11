import { Injectable } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import { Observable } from 'rxjs';

@Injectable({
    providedIn: 'root',
})
export class SystemService {
    private baseUrl = '/api/system';

    constructor(private http: HttpClient) { }

    getConfig(key: string): Observable<any> {
        return this.http.get<any>(`${this.baseUrl}/config/${key}`);
    }

    updateConfig(key: string, value: string): Observable<any> {
        return this.http.put<any>(`${this.baseUrl}/config/${key}`, { configValue: value });
    }
}

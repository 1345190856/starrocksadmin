import { Injectable } from '@angular/core';
import { ApiService } from '../data/api.service';
import { Observable } from 'rxjs';

export interface Application {
    id?: number;
    name: string;
    type: string;
    address: string;
    region: string;
}

@Injectable({
    providedIn: 'root'
})
export class ApplicationService {
    constructor(private api: ApiService) { }

    getApplications(): Observable<Application[]> {
        return this.api.get('/applications');
    }

    createApplication(data: Application): Observable<Application> {
        return this.api.post('/applications', data);
    }

    updateApplication(id: number, data: Partial<Application>): Observable<Application> {
        return this.api.put(`/applications/${id}`, data);
    }

    deleteApplication(id: number): Observable<any> {
        return this.api.delete(`/applications/${id}`);
    }
}

import { NgModule } from '@angular/core';
import { Routes, RouterModule } from '@angular/router';
import { AlertComponent } from './alert.component';
import { AlertDashboardComponent } from './dashboard/alert-dashboard.component';
import { AlertRulesComponent } from './rules/alert-rules.component';

const routes: Routes = [{
    path: '',
    component: AlertComponent,
    children: [
        {
            path: 'dashboard',
            component: AlertDashboardComponent,
        },
        {
            path: 'rules',
            component: AlertRulesComponent,
        },
    ],
}];

@NgModule({
    imports: [RouterModule.forChild(routes)],
    exports: [RouterModule],
})
export class AlertRoutingModule { }

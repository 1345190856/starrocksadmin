import { NgModule } from '@angular/core';
import { Routes, RouterModule } from '@angular/router';

import { DutyPersonnelComponent } from './personnel/personnel.component';

const routes: Routes = [
    {
        path: '',
        children: [
            {
                path: 'personnel',
                component: DutyPersonnelComponent,
            },
            {
                path: '',
                redirectTo: 'personnel',
                pathMatch: 'full',
            },
        ],
    },
];

@NgModule({
    imports: [RouterModule.forChild(routes)],
    exports: [RouterModule],
})
export class DutyRoutingModule { }

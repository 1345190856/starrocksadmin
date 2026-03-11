import { NgModule } from '@angular/core';
import { RouterModule, Routes } from '@angular/router';
import { HeadcountListComponent } from './headcount-list/headcount-list.component';

const routes: Routes = [{
    path: '',
    component: HeadcountListComponent,
}];

@NgModule({
    imports: [RouterModule.forChild(routes)],
    exports: [RouterModule],
})
export class HeadcountRoutingModule { }

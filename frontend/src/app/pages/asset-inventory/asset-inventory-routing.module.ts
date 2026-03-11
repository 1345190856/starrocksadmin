import { NgModule } from '@angular/core';
import { RouterModule, Routes } from '@angular/router';
import { AssetInventoryListComponent } from './asset-inventory-list/asset-inventory-list.component';

const routes: Routes = [
    {
        path: '',
        component: AssetInventoryListComponent,
    },
];

@NgModule({
    imports: [RouterModule.forChild(routes)],
    exports: [RouterModule],
})
export class AssetInventoryRoutingModule { }
